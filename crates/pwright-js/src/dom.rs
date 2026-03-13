//! DOM query JavaScript snippets.

/// Get visible inner text from document body.
pub const GET_INNER_TEXT: &str = "document.body?.innerText || ''";

/// Check if a CSS selector matches any element.
/// Use with `format!` to inject the escaped selector.
pub fn query_selector_exists(selector: &str) -> String {
    format!(
        "!!document.querySelector({})",
        serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_inner_text_is_valid() {
        assert!(!GET_INNER_TEXT.is_empty());
        assert!(GET_INNER_TEXT.contains("innerText"));
    }

    #[test]
    fn test_query_selector_exists_escapes() {
        let js = query_selector_exists("button.submit");
        assert_eq!(js, r#"!!document.querySelector("button.submit")"#);
    }

    #[test]
    fn test_query_selector_exists_quotes() {
        let js = query_selector_exists(r#"[data-id="foo"]"#);
        assert!(js.contains("document.querySelector"));
        // Should be properly JSON-escaped
        assert!(js.contains(r#"\"foo\""#));
    }
}
