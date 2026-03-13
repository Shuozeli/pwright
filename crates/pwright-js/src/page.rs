//! Page-level JavaScript snippets (readyState, location, etc.).

/// Get document.readyState ("loading", "interactive", "complete").
pub const GET_READY_STATE: &str = "document.readyState";

/// Get current page URL.
pub const GET_LOCATION_HREF: &str = "window.location.href";

/// Get document title.
pub const GET_TITLE: &str = "document.title";

/// Get full page HTML.
pub const GET_DOCUMENT_HTML: &str = "document.documentElement.outerHTML";

/// Scroll the page by (dx, dy) pixels.
pub fn scroll_by(dx: i32, dy: i32) -> String {
    format!("window.scrollBy({}, {})", dx, dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_by() {
        assert_eq!(scroll_by(0, 500), "window.scrollBy(0, 500)");
        assert_eq!(scroll_by(-100, 200), "window.scrollBy(-100, 200)");
    }

    #[test]
    fn test_constants_non_empty() {
        assert!(!GET_READY_STATE.is_empty());
        assert!(!GET_LOCATION_HREF.is_empty());
        assert!(!GET_TITLE.is_empty());
        assert!(!GET_DOCUMENT_HTML.is_empty());
    }
}
