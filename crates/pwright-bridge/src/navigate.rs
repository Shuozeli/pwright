//! Navigation — navigate a tab to a URL with wait strategies.

use std::time::Duration;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::{CdpError, Result as CdpResult};

/// Poll interval for ready-state and selector wait loops.
const POLL_INTERVAL_MS: u64 = 200;
/// Poll interval for network idle detection.
const NETWORK_IDLE_POLL_MS: u64 = 250;

/// Wait strategy after navigation.
#[derive(Debug, Clone, Default)]
pub enum WaitStrategy {
    /// Don't wait beyond initial navigation.
    #[default]
    None,
    /// Wait until document.readyState is "interactive" or "complete".
    Dom,
    /// Wait until network is idle (approximation).
    NetworkIdle,
    /// Wait until a CSS selector is visible.
    Selector(String),
}

/// Navigation options.
#[derive(Debug, Clone)]
pub struct NavigateOptions {
    pub wait_for: WaitStrategy,
    pub timeout: Duration,
    pub block_images: bool,
    pub block_media: bool,
}

impl Default for NavigateOptions {
    fn default() -> Self {
        Self {
            wait_for: WaitStrategy::None,
            timeout: Duration::from_secs(30),
            block_images: false,
            block_media: false,
        }
    }
}

/// Navigation result.
#[derive(Debug, Clone)]
pub struct NavigateResult {
    pub tab_id: String,
    pub url: String,
    pub title: String,
}

/// Navigate a tab to the given URL.
pub async fn navigate(
    session: &dyn CdpClient,
    tab_id: &str,
    url: &str,
    opts: &NavigateOptions,
) -> CdpResult<NavigateResult> {
    // Set up resource blocking if requested
    if opts.block_media || opts.block_images {
        let mut patterns = Vec::new();
        if opts.block_media {
            // Media patterns include all image patterns, so skip adding images separately
            patterns.extend(MEDIA_BLOCK_PATTERNS.iter().map(|s| s.to_string()));
        } else if opts.block_images {
            patterns.extend(IMAGE_BLOCK_PATTERNS.iter().map(|s| s.to_string()));
        }
        session.network_set_blocked_urls(&patterns).await?;
    }

    // Navigate
    let nav_result = session.page_navigate(url).await?;
    if let Some(err) = nav_result.get("errorText").and_then(|v| v.as_str())
        && !err.is_empty()
        && err != "net::ERR_HTTP_RESPONSE_CODE_FAILURE"
    {
        return Err(CdpError::NavigationFailed {
            url: url.to_string(),
            reason: err.to_string(),
        });
    }

    // Wait for page to be ready
    wait_for_ready_state(session, &opts.wait_for, opts.timeout).await?;

    // Get final URL and title (log warnings on extraction failure since navigation succeeded)
    let current_url = eval_string(session, "window.location.href")
        .await
        .unwrap_or_else(|| {
            tracing::warn!("failed to extract URL after navigation");
            String::new()
        });
    let title = eval_string(session, "document.title")
        .await
        .unwrap_or_else(|| {
            tracing::warn!("failed to extract title after navigation");
            String::new()
        });

    Ok(NavigateResult {
        tab_id: tab_id.to_string(),
        url: current_url,
        title,
    })
}

/// Wait for the page to reach the desired ready state.
async fn wait_for_ready_state(
    session: &dyn CdpClient,
    strategy: &WaitStrategy,
    timeout: Duration,
) -> CdpResult<()> {
    match strategy {
        WaitStrategy::None => Ok(()),
        WaitStrategy::Dom => poll_ready_state(session, timeout).await,
        WaitStrategy::NetworkIdle => wait_network_idle(session, timeout).await,
        WaitStrategy::Selector(sel) => wait_selector_visible(session, sel, timeout).await,
    }
}

/// Poll document.readyState until "interactive" or "complete".
pub async fn poll_ready_state(session: &dyn CdpClient, timeout: Duration) -> CdpResult<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut interval = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));

    loop {
        interval.tick().await;

        if let Ok(result) = session
            .runtime_evaluate(pwright_js::page::GET_READY_STATE)
            .await
            && let Some(state) =
                crate::evaluate::extract_result_value(&result).and_then(|v| v.as_str())
            && (state == "interactive" || state == "complete")
        {
            return Ok(());
        }

        if tokio::time::Instant::now() > deadline {
            return Err(CdpError::Timeout);
        }
    }
}

/// Approximate network idle: readyState == "complete" and URL stable for 2 checks.
///
/// Limitation: this does NOT monitor actual network activity via CDP. It only
/// checks readyState and URL stability. SPAs that fire additional fetches after
/// `readyState == "complete"` will appear idle prematurely. For those cases,
/// use `WaitStrategy::Selector` with a selector that appears after data loads.
async fn wait_network_idle(session: &dyn CdpClient, timeout: Duration) -> CdpResult<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut interval = tokio::time::interval(Duration::from_millis(NETWORK_IDLE_POLL_MS));
    let mut last_url = String::new();
    let mut idle_checks = 0;

    loop {
        interval.tick().await;

        let ready = eval_string(session, pwright_js::page::GET_READY_STATE).await;
        let cur_url = eval_string(session, pwright_js::page::GET_LOCATION_HREF)
            .await
            .unwrap_or_default();

        if ready.as_deref() == Some("complete") && cur_url == last_url {
            idle_checks += 1;
            if idle_checks >= 2 {
                return Ok(());
            }
        } else {
            idle_checks = 0;
        }
        last_url = cur_url;

        if tokio::time::Instant::now() > deadline {
            return Err(CdpError::Timeout);
        }
    }
}

/// Wait for a CSS selector to become visible.
async fn wait_selector_visible(
    session: &dyn CdpClient,
    selector: &str,
    timeout: Duration,
) -> CdpResult<()> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut interval = tokio::time::interval(Duration::from_millis(POLL_INTERVAL_MS));
    let js = pwright_js::dom::query_selector_exists(selector);

    loop {
        interval.tick().await;

        if let Some(val) = eval_string(session, &js).await
            && val == "true"
        {
            return Ok(());
        }

        if tokio::time::Instant::now() > deadline {
            return Err(CdpError::Timeout);
        }
    }
}

/// Helper: evaluate JS and return the result value as a string.
async fn eval_string(session: &dyn CdpClient, expr: &str) -> Option<String> {
    session.runtime_evaluate(expr).await.ok().and_then(|r| {
        crate::evaluate::extract_result_value(&r).and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| Some(v.to_string()))
        })
    })
}

const IMAGE_BLOCK_PATTERNS: &[&str] = &[
    "*.png", "*.jpg", "*.jpeg", "*.gif", "*.webp", "*.svg", "*.ico",
];

const MEDIA_BLOCK_PATTERNS: &[&str] = &[
    "*.png", "*.jpg", "*.jpeg", "*.gif", "*.webp", "*.svg", "*.ico", "*.mp4", "*.webm", "*.ogg",
    "*.mp3", "*.wav", "*.flac", "*.aac",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigate_options_defaults() {
        let opts = NavigateOptions::default();
        assert_eq!(opts.timeout, Duration::from_secs(30));
        assert!(!opts.block_images);
        assert!(!opts.block_media);
        assert!(matches!(opts.wait_for, WaitStrategy::None));
    }

    #[test]
    fn wait_strategy_default_is_none() {
        let ws = WaitStrategy::default();
        assert!(matches!(ws, WaitStrategy::None));
    }

    #[test]
    fn wait_strategy_selector_stores_string() {
        let ws = WaitStrategy::Selector(".my-element".to_string());
        if let WaitStrategy::Selector(sel) = &ws {
            assert_eq!(sel, ".my-element");
        } else {
            panic!("expected Selector variant");
        }
    }

    #[test]
    fn navigate_result_fields() {
        let result = NavigateResult {
            tab_id: "tab_1".to_string(),
            url: "http://example.com".to_string(),
            title: "Example".to_string(),
        };
        assert_eq!(result.tab_id, "tab_1");
        assert_eq!(result.url, "http://example.com");
        assert_eq!(result.title, "Example");
    }

    #[test]
    fn image_block_patterns_are_complete() {
        assert!(IMAGE_BLOCK_PATTERNS.contains(&"*.png"));
        assert!(IMAGE_BLOCK_PATTERNS.contains(&"*.jpg"));
        assert!(IMAGE_BLOCK_PATTERNS.contains(&"*.jpeg"));
        assert!(IMAGE_BLOCK_PATTERNS.contains(&"*.gif"));
        assert!(IMAGE_BLOCK_PATTERNS.contains(&"*.webp"));
        assert!(IMAGE_BLOCK_PATTERNS.contains(&"*.svg"));
        assert!(IMAGE_BLOCK_PATTERNS.contains(&"*.ico"));
    }

    #[test]
    fn media_block_patterns_include_audio_and_video() {
        assert!(MEDIA_BLOCK_PATTERNS.contains(&"*.mp4"));
        assert!(MEDIA_BLOCK_PATTERNS.contains(&"*.mp3"));
        assert!(MEDIA_BLOCK_PATTERNS.contains(&"*.wav"));
        assert!(MEDIA_BLOCK_PATTERNS.contains(&"*.ogg"));
        // Media patterns also include images
        assert!(MEDIA_BLOCK_PATTERNS.contains(&"*.png"));
    }

    #[test]
    fn media_patterns_are_superset_of_image_patterns() {
        for pat in IMAGE_BLOCK_PATTERNS {
            assert!(
                MEDIA_BLOCK_PATTERNS.contains(pat),
                "media patterns missing: {}",
                pat
            );
        }
    }

    #[test]
    fn navigate_options_custom_values() {
        let opts = NavigateOptions {
            wait_for: WaitStrategy::NetworkIdle,
            timeout: Duration::from_secs(5),
            block_images: true,
            block_media: false,
        };
        assert_eq!(opts.timeout, Duration::from_secs(5));
        assert!(opts.block_images);
        assert!(matches!(opts.wait_for, WaitStrategy::NetworkIdle));
    }

    /// Helper: given NavigateOptions, compute which patterns would be applied.
    /// Mirrors the logic in `navigate()`: media is a superset of images,
    /// so when both are requested, only media patterns are used.
    fn compute_block_patterns(opts: &NavigateOptions) -> Vec<String> {
        let mut patterns = Vec::new();
        if opts.block_media {
            patterns.extend(MEDIA_BLOCK_PATTERNS.iter().map(|s| s.to_string()));
        } else if opts.block_images {
            patterns.extend(IMAGE_BLOCK_PATTERNS.iter().map(|s| s.to_string()));
        }
        patterns
    }

    #[test]
    fn block_media_only_uses_media_patterns() {
        let opts = NavigateOptions {
            block_media: true,
            block_images: false,
            ..Default::default()
        };
        let patterns = compute_block_patterns(&opts);
        assert_eq!(patterns.len(), MEDIA_BLOCK_PATTERNS.len());
        assert!(patterns.iter().any(|p| p == "*.mp4"));
        assert!(patterns.iter().any(|p| p == "*.png"));
    }

    #[test]
    fn block_images_only_uses_image_patterns() {
        let opts = NavigateOptions {
            block_media: false,
            block_images: true,
            ..Default::default()
        };
        let patterns = compute_block_patterns(&opts);
        assert_eq!(patterns.len(), IMAGE_BLOCK_PATTERNS.len());
        assert!(patterns.iter().any(|p| p == "*.png"));
        assert!(!patterns.iter().any(|p| p == "*.mp4"));
    }

    #[test]
    fn both_block_media_and_images_uses_media_only() {
        let opts = NavigateOptions {
            block_media: true,
            block_images: true,
            ..Default::default()
        };
        let patterns = compute_block_patterns(&opts);
        // Media patterns are a superset of image patterns, so only media is used
        assert_eq!(patterns.len(), MEDIA_BLOCK_PATTERNS.len());
        assert!(patterns.iter().any(|p| p == "*.mp4"), "should block media");
        assert!(
            patterns.iter().any(|p| p == "*.jpg"),
            "should block images via media superset"
        );
    }

    #[test]
    fn no_blocking_produces_empty_patterns() {
        let opts = NavigateOptions::default();
        let patterns = compute_block_patterns(&opts);
        assert!(patterns.is_empty());
    }

    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn poll_ready_state_succeeds_when_complete() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "complete"}
        }));

        poll_ready_state(&mock, Duration::from_secs(2))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn poll_ready_state_succeeds_when_interactive() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "interactive"}
        }));

        poll_ready_state(&mock, Duration::from_secs(2))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn poll_ready_state_times_out_when_loading() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "loading"}
        }));

        let result = poll_ready_state(&mock, Duration::from_millis(400)).await;
        assert!(matches!(result, Err(CdpError::Timeout)));
    }

    #[tokio::test]
    async fn wait_for_ready_state_none_returns_immediately() {
        let mock = MockCdpClient::new();
        // No evaluate response configured — should never be called
        wait_for_ready_state(&mock, &WaitStrategy::None, Duration::from_secs(1))
            .await
            .unwrap();
        assert!(mock.method_names().is_empty());
    }
}
