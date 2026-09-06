use std::sync::Arc;
use std::time::Duration;

use pwright_backend_cdp::{
    CdpNetworkEventAdapter, CdpNetworkEventEffect, CdpNetworkEventProductionProfile,
};
use pwright_browser_model::{REQUEST_OBJECT_KIND, RESPONSE_OBJECT_KIND};
use pwright_browser_provider::AttachBrowserProvider;
use pwright_cdp::CdpSession;
use pwright_cdp::connection::ConnectionConfig;
use pwright_core::{
    AtomicObjectIdSource, ControlMessage, ObjectId, ProductionResourceLedger, ResourceClass,
    ResourceLimits, ResourceScope,
};
use pwright_network_model::NetworkLifecycleLimits;
use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("temporary target {target_id} remained live after context disposal");
}

#[tokio::test]
#[ignore = "requires dedicated Chrome, PWRIGHT_CDP_WS_URL, TAILSCALE_IP, and TAILSCALE_HOST"]
async fn bounded_network_adapter_observes_a_real_http_lifecycle_without_reading_its_body() {
    // Arrange
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let tailscale_ip = std::env::var("TAILSCALE_IP").expect("TAILSCALE_IP must be set");
    let tailscale_host = std::env::var("TAILSCALE_HOST").expect("TAILSCALE_HOST must be set");
    let listener = tokio::net::TcpListener::bind(format!("{tailscale_ip}:0"))
        .await
        .expect("fixture server should bind to the Tailscale interface");
    let port = listener
        .local_addr()
        .expect("fixture address should exist")
        .port();
    let url = format!("http://{tailscale_host}:{port}/network-lifecycle");
    let fixture = tokio::spawn(async move {
        let (mut socket, _) = listener
            .accept()
            .await
            .expect("Chrome should connect to the fixture");
        let mut request = [0_u8; 4096];
        let bytes = socket
            .read(&mut request)
            .await
            .expect("fixture request should be readable");
        assert!(String::from_utf8_lossy(&request[..bytes]).starts_with("GET /network-lifecycle "));
        let body = b"<title>bounded network event</title>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("fixture headers should write");
        socket
            .write_all(body)
            .await
            .expect("fixture body should write");
        socket
            .shutdown()
            .await
            .expect("fixture should close cleanly");
    });

    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without taking process ownership");
    let connection = Arc::clone(attachment.connection());
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
    let page = CdpSession::new(
        Arc::clone(&connection),
        session_id.clone(),
        target_id.clone(),
    );
    let limits = NetworkLifecycleLimits::new(16, 128, 2048, 32, 64, 64, 8192, 1024, 128, 256);
    let profile = CdpNetworkEventProductionProfile::new(limits, ResourceScope::extension(95, 1))
        .expect("production profile should validate");
    let requirements = profile.requirements();
    let backend_state = requirements.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            backend_state.global,
            backend_state.per_scope,
        ),
        requirements,
    )
    .expect("production resource plan should validate");
    let mut adapter = CdpNetworkEventAdapter::new_with_production_resources(
        ObjectId::new(3),
        session_id,
        Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
        profile,
        &resources,
    )
    .expect("network adapter should reserve its complete state bound");
    let mut events = connection.subscribe_events();
    page.network_enable()
        .await
        .expect("Network events should enable before navigation");

    // Act
    page.page_navigate(&url)
        .await
        .expect("fixture navigation should start");
    let observed = tokio::time::timeout(Duration::from_secs(5), async {
        let mut lifecycle = Vec::new();
        let mut semantic = Vec::new();
        let mut target_request = None;
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain live");
            let batch = adapter
                .map_event(&event)
                .expect("real CDP event should normalize");
            lifecycle.extend_from_slice(batch.lifecycle());
            for event in batch.events() {
                match event {
                    CdpNetworkEventEffect::Request(event) if event.payload().url == url => {
                        target_request = Some(event.payload().request);
                        semantic.push("request");
                    }
                    CdpNetworkEventEffect::Response(event)
                        if Some(event.payload().request) == target_request =>
                    {
                        assert_eq!(event.payload().status, 200);
                        semantic.push("response");
                    }
                    CdpNetworkEventEffect::RequestFinished(event)
                        if Some(event.payload().request.request) == target_request =>
                    {
                        assert!(event.payload().response.is_some());
                        semantic.push("requestfinished");
                        return (lifecycle, semantic);
                    }
                    CdpNetworkEventEffect::RequestFailed(event)
                        if Some(event.payload().request.request) == target_request =>
                    {
                        panic!("fixture request failed: {}", event.payload().failure_text);
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .expect("real HTTP lifecycle should finish within the deadline");
    fixture.await.expect("fixture task should finish");
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;

    // Assert
    assert_eq!(adapter.active_request_count(), 0);
    assert_eq!(adapter.retained_native_id_capacity(), 0);
    assert_eq!(
        resources
            .metrics()
            .expect("resource metrics should be available")
            .class(ResourceClass::BackendStateBytes)
            .current,
        u64::try_from(profile.maximum_retained_bytes()).unwrap()
    );
    assert_eq!(
        observed.0,
        vec![
            ControlMessage::Create {
                object_id: ObjectId::new(10),
                parent: Some(ObjectId::new(3)),
                kind: REQUEST_OBJECT_KIND.tag,
            },
            ControlMessage::Create {
                object_id: ObjectId::new(11),
                parent: Some(ObjectId::new(10)),
                kind: RESPONSE_OBJECT_KIND.tag,
            },
        ]
    );
    assert_eq!(observed.1, vec!["request", "response", "requestfinished"]);
    assert!(adapter.drain_active().is_empty());
    drop(adapter);
    assert_eq!(
        resources
            .metrics()
            .expect("resource metrics should be available")
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after owned context disposal");
}

#[tokio::test]
#[ignore = "requires dedicated Chrome, PWRIGHT_CDP_WS_URL, TAILSCALE_IP, and TAILSCALE_HOST"]
async fn real_chrome_distinguishes_redirect_http_error_and_transport_failure() {
    // Arrange
    let endpoint = CdpWebSocketEndpoint::parse(
        std::env::var("PWRIGHT_CDP_WS_URL")
            .expect("PWRIGHT_CDP_WS_URL must point to the dedicated Chrome CDP profile"),
    )
    .expect("dedicated Chrome endpoint should be valid");
    let tailscale_ip = std::env::var("TAILSCALE_IP").expect("TAILSCALE_IP must be set");
    let tailscale_host = std::env::var("TAILSCALE_HOST").expect("TAILSCALE_HOST must be set");
    let listener = tokio::net::TcpListener::bind(format!("{tailscale_ip}:0"))
        .await
        .expect("fixture server should bind to the Tailscale interface");
    let port = listener
        .local_addr()
        .expect("fixture address should exist")
        .port();
    let redirect_url = format!("http://{tailscale_host}:{port}/redirect");
    let server_error_url = format!("http://{tailscale_host}:{port}/server-error");
    let aborted_url = format!("http://{tailscale_host}:{port}/aborted");
    let redirect_location = server_error_url.clone();
    let fixture = tokio::spawn(async move {
        let mut expected_responses = 0usize;
        while expected_responses < 3 {
            let (mut socket, _) = listener
                .accept()
                .await
                .expect("Chrome should connect to the fixture");
            let mut request = [0_u8; 4096];
            let bytes = socket
                .read(&mut request)
                .await
                .expect("fixture request should be readable");
            let request = String::from_utf8_lossy(&request[..bytes]);
            let path = request
                .split_whitespace()
                .nth(1)
                .expect("HTTP request path should exist");
            match path {
                "/redirect" => {
                    let response = format!(
                        "HTTP/1.1 302 Found\r\nLocation: {redirect_location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("redirect response should write");
                    expected_responses += 1;
                }
                "/server-error" => {
                    let body = b"<link rel=\"icon\" href=\"data:,\"><title>server error</title>";
                    let response = format!(
                        "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    socket
                        .write_all(response.as_bytes())
                        .await
                        .expect("HTTP error headers should write");
                    socket
                        .write_all(body)
                        .await
                        .expect("HTTP error body should write");
                    expected_responses += 1;
                }
                "/aborted" => {
                    socket
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 1048576\r\nConnection: close\r\n\r\npartial",
                        )
                        .await
                        .expect("partial response should write");
                    expected_responses += 1;
                }
                _ => {
                    socket
                        .write_all(
                            b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        )
                        .await
                        .expect("incidental request should receive a bounded response");
                }
            }
            socket
                .shutdown()
                .await
                .expect("fixture connection should close");
        }
    });

    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .expect("provider should attach without taking process ownership");
    let connection = Arc::clone(attachment.connection());
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
    let page = CdpSession::new(
        Arc::clone(&connection),
        session_id.clone(),
        target_id.clone(),
    );
    let limits = NetworkLifecycleLimits::new(16, 128, 2048, 32, 64, 64, 8192, 1024, 128, 256);
    let mut adapter = CdpNetworkEventAdapter::new(
        ObjectId::new(3),
        session_id,
        Arc::new(AtomicObjectIdSource::new(10).expect("object IDs should validate")),
        limits,
    )
    .expect("network adapter should validate");
    let mut events = connection.subscribe_events();
    page.network_enable()
        .await
        .expect("Network events should enable before navigation");

    // Act: a redirect reuses CDP's native ID but must close the old portable
    // Request/Response pair before creating the next Request. HTTP 500 remains
    // a successful transport lifecycle.
    page.page_navigate(&redirect_url)
        .await
        .expect("redirect navigation should start");
    let redirect_trace = tokio::time::timeout(Duration::from_secs(5), async {
        let mut trace = Vec::new();
        let mut first_request = None;
        let mut second_request = None;
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain live");
            let batch = adapter
                .map_event(&event)
                .expect("real redirect event should normalize");
            for event in batch.events() {
                match event {
                    CdpNetworkEventEffect::Request(event)
                        if event.payload().url == redirect_url =>
                    {
                        first_request = Some(event.payload().request);
                        trace.push("request:redirect");
                    }
                    CdpNetworkEventEffect::Response(event)
                        if Some(event.payload().request) == first_request =>
                    {
                        assert_eq!(event.payload().status, 302);
                        trace.push("response:302");
                    }
                    CdpNetworkEventEffect::RequestFinished(event)
                        if Some(event.payload().request.request) == first_request =>
                    {
                        trace.push("requestfinished:redirect");
                    }
                    CdpNetworkEventEffect::Request(event)
                        if event.payload().url == server_error_url =>
                    {
                        assert_eq!(event.payload().redirected_from, first_request);
                        second_request = Some(event.payload().request);
                        trace.push("request:server-error");
                    }
                    CdpNetworkEventEffect::Response(event)
                        if Some(event.payload().request) == second_request =>
                    {
                        assert_eq!(event.payload().status, 500);
                        trace.push("response:500");
                    }
                    CdpNetworkEventEffect::RequestFinished(event)
                        if Some(event.payload().request.request) == second_request =>
                    {
                        trace.push("requestfinished:server-error");
                        return trace;
                    }
                    CdpNetworkEventEffect::RequestFailed(event)
                        if Some(event.payload().request.request) == first_request
                            || Some(event.payload().request.request) == second_request =>
                    {
                        panic!(
                            "redirect/HTTP error unexpectedly failed: {}",
                            event.payload().failure_text
                        );
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .expect("redirect and HTTP error should finish within the deadline");

    // Act: a truncated response is a transport failure even though response
    // headers arrived first.
    page.page_navigate(&aborted_url)
        .await
        .expect("aborted navigation should start");
    let (failed, observed_response) = tokio::time::timeout(Duration::from_secs(5), async {
        let mut request = None;
        let mut response = None;
        loop {
            let event = events
                .recv()
                .await
                .expect("CDP event stream should remain live");
            let batch = adapter
                .map_event(&event)
                .expect("real failure event should normalize");
            for event in batch.events() {
                match event {
                    CdpNetworkEventEffect::Request(event) if event.payload().url == aborted_url => {
                        request = Some(event.payload().request);
                    }
                    CdpNetworkEventEffect::Response(event)
                        if Some(event.payload().request) == request =>
                    {
                        assert_eq!(event.payload().status, 200);
                        response = Some(event.payload().response);
                    }
                    CdpNetworkEventEffect::RequestFailed(event)
                        if Some(event.payload().request.request) == request =>
                    {
                        return (event.payload().clone(), response);
                    }
                    CdpNetworkEventEffect::RequestFinished(event)
                        if Some(event.payload().request.request) == request =>
                    {
                        panic!("truncated response unexpectedly finished");
                    }
                    _ => {}
                }
            }
        }
    })
    .await
    .expect("truncated response should fail within the deadline");
    fixture.await.expect("fixture task should finish");
    dispose_context_and_verify_target(&browser, &browser_context_id, &target_id).await;

    // Assert
    assert_eq!(
        redirect_trace,
        vec![
            "request:redirect",
            "response:302",
            "requestfinished:redirect",
            "request:server-error",
            "response:500",
            "requestfinished:server-error",
        ]
    );
    assert_eq!(
        failed.response.as_ref().map(|response| response.response),
        observed_response
    );
    assert!(!failed.failure_text.is_empty());
    assert_eq!(adapter.active_request_count(), 0);
    assert_eq!(adapter.retained_native_id_capacity(), 0);
    assert!(adapter.drain_active().is_empty());
    connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .expect("external browser should remain alive after owned context disposal");
}
