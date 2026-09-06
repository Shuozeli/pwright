use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use pwright_bridge::navigate::{NavigateOptions, WaitStrategy};
use pwright_bridge::{Browser, BrowserConfig};
use pwright_cdp::CdpClient;
use pwright_cdp::connection::{CdpError, Result as CdpResult};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct CaptureUrlOptions {
    pub cdp_url: String,
    pub ephemeral_proxy_server: Option<String>,
    pub ephemeral_proxy_bypass_list: Option<String>,
    pub url: String,
    pub context_url: Option<String>,
    pub dest: PathBuf,
    pub max_bytes: u64,
    pub timeout_ms: u64,
    pub json: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RedirectHop {
    url: String,
    status: i64,
}

#[derive(Debug, Clone, Serialize)]
struct CaptureUrlResult {
    requested_url: String,
    context_url: Option<String>,
    final_url: String,
    status: i64,
    status_text: String,
    mime_type: String,
    response_headers: BTreeMap<String, String>,
    redirect_chain: Vec<RedirectHop>,
    network_request_id: Option<String>,
    bytes_written: u64,
    destination: String,
    transport: &'static str,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct CaptureCleanupWarnings {
    fetch_disable_error: Option<String>,
    tab_close_error: Option<String>,
}

#[derive(Debug, Clone)]
struct PausedResponse {
    fetch_request_id: String,
    network_request_id: Option<String>,
    frame_id: String,
    resource_type: String,
    url: String,
    status: i64,
    status_text: String,
    headers: BTreeMap<String, String>,
}

impl PausedResponse {
    fn from_event(params: &Value) -> Option<Self> {
        let status = params.get("responseStatusCode")?.as_i64()?;
        let request = params.get("request")?;
        Some(Self {
            fetch_request_id: params.get("requestId")?.as_str()?.to_owned(),
            network_request_id: params
                .get("networkId")
                .and_then(Value::as_str)
                .map(str::to_owned),
            frame_id: params.get("frameId")?.as_str()?.to_owned(),
            resource_type: params
                .get("resourceType")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            url: request.get("url")?.as_str()?.to_owned(),
            status,
            status_text: params
                .get("responseStatusText")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            headers: header_entries(params.get("responseHeaders")),
        })
    }

    fn is_redirect(&self) -> bool {
        (300..400).contains(&self.status)
    }

    fn mime_type(&self) -> String {
        header_value(&self.headers, "content-type")
            .and_then(|value| value.split(';').next())
            .unwrap_or_default()
            .trim()
            .to_owned()
    }
}

#[derive(Debug, Default)]
struct MainDocumentMatcher {
    requested_url: String,
    capture_frame_id: Option<String>,
}

impl MainDocumentMatcher {
    fn new(requested_url: String) -> Self {
        Self {
            requested_url,
            capture_frame_id: None,
        }
    }

    fn matches(&mut self, response: &PausedResponse) -> bool {
        if response.resource_type != "Document" {
            return false;
        }
        if let Some(frame_id) = &self.capture_frame_id {
            return frame_id == &response.frame_id;
        }
        if same_url(&self.requested_url, &response.url) {
            self.capture_frame_id = Some(response.frame_id.clone());
            return true;
        }
        false
    }
}

pub async fn capture_url(options: CaptureUrlOptions) -> Result<()> {
    validate_options(&options)?;
    let config = BrowserConfig {
        cdp_url: options.cdp_url.clone(),
        command_timeout_ms: options.timeout_ms,
        navigate_timeout_ms: options.timeout_ms,
        tab_close_via_http: options.cdp_url.starts_with("http://")
            || options.cdp_url.starts_with("https://"),
        ephemeral_proxy_server: options.ephemeral_proxy_server.clone(),
        ephemeral_proxy_bypass_list: options.ephemeral_proxy_bypass_list.clone(),
        ..Default::default()
    };
    let browser = Browser::connect(config)
        .await
        .context("failed to connect to Chrome")?;
    let lease_root = crate::commands::ephemeral_lease_root();
    crate::commands::recover_ephemeral_script_tabs(&browser, &options.cdp_url, &lease_root).await?;
    let (handle, lease_path) = browser
        .new_tab_with_created_target("about:blank", |target_id| {
            crate::state::save_ephemeral_tab_lease(&lease_root, &options.cdp_url, target_id)
                .map_err(|error| {
                    CdpError::Other(format!(
                        "cannot persist ephemeral capture-tab lease: {error}"
                    ))
                })
        })
        .await
        .context("failed to create capture tab")?;
    let session = handle.session();

    let capture_result = capture_in_tab(session.as_ref(), "capture-tab", &options).await;
    let disable_result = session.fetch_disable().await;
    let cleanup_result = handle.close().await;
    if cleanup_result.is_ok() {
        crate::state::remove_ephemeral_tab_lease(&lease_path)?;
    }

    let (result, cleanup_warnings) =
        finish_capture(capture_result, disable_result, cleanup_result)?;
    if let Some(error) = cleanup_warnings.fetch_disable_error {
        tracing::warn!(
            event = "capture_url_fetch_cleanup_failed_after_persist",
            destination = %result.destination,
            error,
            "capture bytes are durable; Fetch cleanup failure is non-fatal"
        );
    }
    if let Some(error) = cleanup_warnings.tab_close_error {
        tracing::warn!(
            event = "capture_url_tab_cleanup_deferred",
            destination = %result.destination,
            lease = %lease_path.display(),
            error,
            "capture bytes are durable; exact tab lease is retained for recovery"
        );
    }
    emit_result(&result, options.json)
}

fn finish_capture<T>(
    capture_result: Result<T>,
    disable_result: CdpResult<()>,
    cleanup_result: CdpResult<()>,
) -> Result<(T, CaptureCleanupWarnings)> {
    let result = capture_result?;
    Ok((
        result,
        CaptureCleanupWarnings {
            fetch_disable_error: disable_result.err().map(|error| error.to_string()),
            tab_close_error: cleanup_result.err().map(|error| error.to_string()),
        },
    ))
}

async fn capture_in_tab(
    session: &dyn CdpClient,
    tab_id: &str,
    options: &CaptureUrlOptions,
) -> Result<CaptureUrlResult> {
    if let Some(context_url) = options
        .context_url
        .as_deref()
        .filter(|context_url| !same_url(context_url, &options.url))
    {
        pwright_bridge::navigate::navigate(
            session,
            tab_id,
            context_url,
            &NavigateOptions {
                wait_for: WaitStrategy::Dom,
                timeout: Duration::from_millis(options.timeout_ms),
                ..Default::default()
            },
        )
        .await
        .context("failed to load browser context URL before capture")?;
    }
    session
        .fetch_enable_response_stage()
        .await
        .context("failed to enable Chrome response-stage interception")?;
    let mut events = session.subscribe_events();
    let capture = tokio::time::timeout(
        Duration::from_millis(options.timeout_ms),
        capture_main_document(session, &mut events, options),
    )
    .await
    .context("timed out waiting for the navigated response body")??;
    write_new_file(&options.dest, &capture.bytes)?;
    let mime_type = capture.response.mime_type();

    Ok(CaptureUrlResult {
        requested_url: options.url.clone(),
        context_url: options.context_url.clone(),
        final_url: capture.response.url,
        status: capture.response.status,
        status_text: capture.response.status_text,
        mime_type,
        response_headers: capture.response.headers,
        redirect_chain: capture.redirects,
        network_request_id: capture.response.network_request_id,
        bytes_written: capture.bytes.len() as u64,
        destination: options.dest.display().to_string(),
        transport: "cdp_fetch_response_body",
    })
}

struct CapturedBody {
    response: PausedResponse,
    redirects: Vec<RedirectHop>,
    bytes: Vec<u8>,
}

async fn capture_main_document(
    session: &dyn CdpClient,
    events: &mut tokio::sync::broadcast::Receiver<pwright_cdp::events::CdpEvent>,
    options: &CaptureUrlOptions,
) -> Result<CapturedBody> {
    let navigation = session.page_navigate(&options.url);
    tokio::pin!(navigation);
    let mut navigation_finished = false;
    let mut matcher = MainDocumentMatcher::new(options.url.clone());
    let mut redirects = Vec::new();

    loop {
        tokio::select! {
            navigation_result = &mut navigation, if !navigation_finished => {
                let result = navigation_result.context("Chrome navigation command failed")?;
                if let Some(error) = result.get("errorText").and_then(Value::as_str) {
                    anyhow::bail!("Chrome navigation failed: {error}");
                }
                navigation_finished = true;
            }
            event_result = events.recv() => {
                let event = receive_event(event_result)?;
                if event.method != "Fetch.requestPaused" {
                    continue;
                }
                let Some(response) = PausedResponse::from_event(&event.params) else {
                    continue_unmatched_request(session, &event.params).await?;
                    continue;
                };
                if !matcher.matches(&response) {
                    session
                        .fetch_continue_request(&response.fetch_request_id)
                        .await
                        .context("failed to continue an unrelated intercepted response")?;
                    continue;
                }
                if response.is_redirect() {
                    redirects.push(RedirectHop {
                        url: response.url,
                        status: response.status,
                    });
                    session
                        .fetch_continue_request(&response.fetch_request_id)
                        .await
                        .context("failed to continue a redirect response")?;
                    continue;
                }
                enforce_header_limit(&response.headers, options.max_bytes)?;
                let body_result = session
                    .fetch_get_response_body(&response.fetch_request_id)
                    .await
                    .context("Chrome could not return the raw intercepted response body");
                let continue_result = session
                    .fetch_continue_request(&response.fetch_request_id)
                    .await
                    .context("failed to continue the captured response");
                let body = body_result?;
                continue_result?;
                let bytes = decode_body(body.body, body.base64_encoded, options.max_bytes)?;
                return Ok(CapturedBody {
                    response,
                    redirects,
                    bytes,
                });
            }
        }
    }
}

fn receive_event(
    event: Result<pwright_cdp::events::CdpEvent, tokio::sync::broadcast::error::RecvError>,
) -> Result<pwright_cdp::events::CdpEvent> {
    match event {
        Ok(event) => Ok(event),
        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
            anyhow::bail!("Chrome network event receiver lagged by {count} events")
        }
        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
            anyhow::bail!("Chrome network event stream closed before capture completed")
        }
    }
}

async fn continue_unmatched_request(session: &dyn CdpClient, params: &Value) -> Result<()> {
    if let Some(request_id) = params.get("requestId").and_then(Value::as_str) {
        session
            .fetch_continue_request(request_id)
            .await
            .context("failed to continue an unmatched intercepted request")?;
    }
    Ok(())
}

fn validate_options(options: &CaptureUrlOptions) -> Result<()> {
    let parsed = reqwest::Url::parse(&options.url).context("invalid capture URL")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        anyhow::bail!("capture URL must use http or https");
    }
    if let Some(context_url) = &options.context_url {
        let parsed = reqwest::Url::parse(context_url).context("invalid browser context URL")?;
        if !matches!(parsed.scheme(), "http" | "https") {
            anyhow::bail!("browser context URL must use http or https");
        }
    }
    if options.max_bytes == 0 {
        anyhow::bail!("--max-bytes must be greater than zero");
    }
    if options.timeout_ms == 0 {
        anyhow::bail!("--timeout must be greater than zero");
    }
    Ok(())
}

fn decode_body(body: String, base64_encoded: bool, max_bytes: u64) -> Result<Vec<u8>> {
    if base64_encoded {
        let upper_bound = ((body.len() as u64).saturating_add(3) / 4).saturating_mul(3);
        if upper_bound > max_bytes.saturating_add(2) {
            anyhow::bail!("decoded response exceeds --max-bytes {max_bytes}");
        }
        let decoded = BASE64
            .decode(body)
            .context("Chrome returned an invalid base64 response body")?;
        ensure_size(decoded.len(), max_bytes)?;
        Ok(decoded)
    } else {
        let bytes = body.into_bytes();
        ensure_size(bytes.len(), max_bytes)?;
        Ok(bytes)
    }
}

fn ensure_size(length: usize, max_bytes: u64) -> Result<()> {
    if length as u64 > max_bytes {
        anyhow::bail!("response size {length} exceeds --max-bytes {max_bytes}");
    }
    Ok(())
}

fn enforce_header_limit(headers: &BTreeMap<String, String>, max_bytes: u64) -> Result<()> {
    if let Some(content_length) =
        header_value(headers, "content-length").and_then(|value| value.parse::<u64>().ok())
        && content_length > max_bytes
    {
        anyhow::bail!("response Content-Length {content_length} exceeds --max-bytes {max_bytes}");
    }
    Ok(())
}

fn write_new_file(destination: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = destination.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create destination directory {}",
                parent.display()
            )
        })?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .with_context(|| {
            format!(
                "failed to create destination {} without overwriting",
                destination.display()
            )
        })?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = std::fs::remove_file(destination);
        return Err(error)
            .with_context(|| format!("failed to write destination {}", destination.display()));
    }
    Ok(())
}

fn emit_result(result: &CaptureUrlResult, json: bool) -> Result<()> {
    if json {
        println!(
            "{}",
            serde_json::to_string(result).context("failed to serialize capture result")?
        );
    } else {
        crate::output::ok(&format!(
            "Captured {} bytes from {} to {}",
            result.bytes_written, result.final_url, result.destination
        ));
    }
    Ok(())
}

fn header_entries(value: Option<&Value>) -> BTreeMap<String, String> {
    let Some(entries) = value.and_then(Value::as_array) else {
        return BTreeMap::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            Some((
                entry.get("name")?.as_str()?.to_owned(),
                entry.get("value")?.as_str()?.to_owned(),
            ))
        })
        .collect()
}

fn header_value<'a>(headers: &'a BTreeMap<String, String>, expected_name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(expected_name))
        .map(|(_, value)| value.as_str())
}

fn same_url(left: &str, right: &str) -> bool {
    match (reqwest::Url::parse(left), reqwest::Url::parse(right)) {
        (Ok(mut left), Ok(mut right)) => {
            left.set_fragment(None);
            right.set_fragment(None);
            left == right
        }
        _ => left == right,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn paused_document(url: &str, status: i64, frame_id: &str) -> Value {
        json!({
            "requestId": "fetch-1",
            "networkId": "network-1",
            "frameId": frame_id,
            "resourceType": "Document",
            "request": {"url": url},
            "responseStatusCode": status,
            "responseStatusText": "OK",
            "responseHeaders": [
                {"name": "Content-Type", "value": "application/pdf"},
                {"name": "Content-Length", "value": "512"}
            ]
        })
    }

    #[test]
    fn matcher_claims_exact_navigation_and_follows_same_frame_redirect() {
        // Arrange
        let mut matcher = MainDocumentMatcher::new("https://issuer.example/report".to_owned());
        let first = PausedResponse::from_event(&paused_document(
            "https://issuer.example/report",
            302,
            "frame-1",
        ))
        .expect("valid paused response");
        let redirected = PausedResponse::from_event(&paused_document(
            "https://cdn.example/report.pdf",
            200,
            "frame-1",
        ))
        .expect("valid paused response");

        // Act
        let first_matches = matcher.matches(&first);
        let redirected_matches = matcher.matches(&redirected);

        // Assert
        assert!(first_matches);
        assert!(redirected_matches);
    }

    #[test]
    fn matcher_rejects_subresources_and_other_frames() {
        // Arrange
        let mut matcher = MainDocumentMatcher::new("https://issuer.example/report".to_owned());
        let first = PausedResponse::from_event(&paused_document(
            "https://issuer.example/report",
            302,
            "frame-1",
        ))
        .expect("valid paused response");
        assert!(matcher.matches(&first));
        let mut subresource = paused_document("https://issuer.example/report", 200, "frame-1");
        subresource["resourceType"] = json!("Script");
        let subresource = PausedResponse::from_event(&subresource).expect("valid paused response");
        let other_frame = PausedResponse::from_event(&paused_document(
            "https://cdn.example/report.pdf",
            200,
            "frame-2",
        ))
        .expect("valid paused response");

        // Act
        let subresource_matches = matcher.matches(&subresource);
        let other_frame_matches = matcher.matches(&other_frame);

        // Assert
        assert!(!subresource_matches);
        assert!(!other_frame_matches);
    }

    #[test]
    fn paused_response_preserves_raw_headers_and_network_identity() {
        // Arrange
        let event = paused_document("https://cdn.example/report.pdf", 200, "frame-1");

        // Act
        let response = PausedResponse::from_event(&event).expect("valid paused response");

        // Assert
        assert_eq!(response.fetch_request_id, "fetch-1");
        assert_eq!(response.network_request_id.as_deref(), Some("network-1"));
        assert_eq!(response.mime_type(), "application/pdf");
        assert_eq!(
            header_value(&response.headers, "content-length"),
            Some("512")
        );
    }

    #[test]
    fn response_content_length_over_limit_is_rejected_before_body_fetch() {
        // Arrange
        let headers = BTreeMap::from([("Content-Length".to_owned(), "2048".to_owned())]);

        // Act
        let result = enforce_header_limit(&headers, 1_024);

        // Assert
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("2048"));
    }

    #[test]
    fn decoded_binary_body_respects_exact_limit() -> Result<()> {
        // Arrange
        let encoded = BASE64.encode(b"%PDF");

        // Act
        let bytes = decode_body(encoded, true, 4)?;

        // Assert
        assert_eq!(bytes, b"%PDF");
        Ok(())
    }

    #[test]
    fn destination_is_not_overwritten() -> Result<()> {
        // Arrange
        let directory = tempfile::tempdir()?;
        let destination = directory.path().join("report.pdf");
        std::fs::write(&destination, b"original")?;

        // Act
        let result = write_new_file(&destination, b"replacement");

        // Assert
        assert!(result.is_err());
        assert_eq!(std::fs::read(destination)?, b"original");
        Ok(())
    }

    #[test]
    fn successful_capture_survives_post_persist_cleanup_failures() -> Result<()> {
        // Arrange
        let capture_result = Ok(42_u64);
        let disable_result = Err(CdpError::Other("Fetch session closed".to_owned()));
        let cleanup_result = Err(CdpError::Other(
            "Chrome target list returned 502".to_owned(),
        ));

        // Act
        let (value, warnings) = finish_capture(capture_result, disable_result, cleanup_result)?;

        // Assert
        assert_eq!(value, 42);
        assert_eq!(
            warnings.fetch_disable_error.as_deref(),
            Some("Fetch session closed")
        );
        assert_eq!(
            warnings.tab_close_error.as_deref(),
            Some("Chrome target list returned 502")
        );
        Ok(())
    }

    #[test]
    fn failed_capture_remains_failed_even_when_cleanup_also_fails() {
        // Arrange
        let capture_result = Err(anyhow::anyhow!("response body was not captured"));
        let disable_result = Err(CdpError::Other("Fetch session closed".to_owned()));
        let cleanup_result = Err(CdpError::Other(
            "Chrome target list returned 502".to_owned(),
        ));

        // Act
        let result: Result<(u64, CaptureCleanupWarnings)> =
            finish_capture(capture_result, disable_result, cleanup_result);

        // Assert
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("response body was not captured")
        );
    }

    #[test]
    fn url_matching_ignores_fragments_but_not_hosts() {
        // Arrange
        let requested = "https://issuer.example/report#download";

        // Act
        let same = same_url(requested, "https://issuer.example/report");
        let different = same_url(requested, "https://cdn.example/report");

        // Assert
        assert!(same);
        assert!(!different);
    }
}
