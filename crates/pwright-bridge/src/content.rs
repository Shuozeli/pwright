//! Content extraction — screenshot, PDF, text.

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use serde_json::json;

/// Screenshot format.
#[derive(Debug, Clone, Default)]
pub enum ScreenshotFormat {
    #[default]
    Png,
    Jpeg(i32), // quality 0-100
    Webp(i32),
}

/// Take a screenshot of the current page. Returns base64-encoded data.
pub async fn take_screenshot(
    session: &dyn CdpClient,
    format: &ScreenshotFormat,
    full_page: bool,
) -> CdpResult<String> {
    let (fmt, quality) = match format {
        ScreenshotFormat::Png => ("png", None),
        ScreenshotFormat::Jpeg(q) => ("jpeg", Some(*q)),
        ScreenshotFormat::Webp(q) => ("webp", Some(*q)),
    };
    session
        .page_capture_screenshot(fmt, quality, full_page)
        .await
}

/// Print the page to PDF. Returns base64-encoded PDF data.
pub async fn get_pdf(session: &dyn CdpClient) -> CdpResult<String> {
    session
        .page_print_to_pdf(json!({
            "printBackground": true,
        }))
        .await
}

/// Extract visible text content from the page.
pub async fn get_text(session: &dyn CdpClient) -> CdpResult<String> {
    let result = session
        .runtime_evaluate(pwright_js::dom::GET_INNER_TEXT)
        .await?;
    let text = result
        .get("result")
        .and_then(|r| r.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    Ok(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_take_screenshot_png() {
        let mock = MockCdpClient::new();
        mock.set_screenshot_response("iVBORw0KGgo=".to_string());

        let result = take_screenshot(&mock, &ScreenshotFormat::Png, false)
            .await
            .unwrap();
        assert_eq!(result, "iVBORw0KGgo=");

        let calls = mock.calls_for("Page.captureScreenshot");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0]["format"], "png");
        assert_eq!(calls[0].args[0]["fullPage"], false);
    }

    #[tokio::test]
    async fn test_take_screenshot_jpeg_with_quality() {
        let mock = MockCdpClient::new();
        take_screenshot(&mock, &ScreenshotFormat::Jpeg(80), true)
            .await
            .unwrap();

        let calls = mock.calls_for("Page.captureScreenshot");
        assert_eq!(calls[0].args[0]["format"], "jpeg");
        assert_eq!(calls[0].args[0]["quality"], 80);
        assert_eq!(calls[0].args[0]["fullPage"], true);
    }

    #[tokio::test]
    async fn test_get_text_extracts_inner_text() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"value": "Hello World"}
        }));

        let text = get_text(&mock).await.unwrap();
        assert_eq!(text, "Hello World");
    }

    #[tokio::test]
    async fn test_get_pdf() {
        let mock = MockCdpClient::new();
        mock.set_pdf_response("JVBER123".to_string());

        let result = get_pdf(&mock).await.unwrap();
        assert_eq!(result, "JVBER123");
    }
}
