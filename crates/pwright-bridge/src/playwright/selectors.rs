//! Selector parsing and resolution.
//!
//! Converts Playwright-style selectors into DOM operations.
//! Supports CSS selectors and special prefixes:
//!   - `__pw_text=<text>` — find element by text content (substring)
//!   - `__pw_text_exact=<text>` — find element by exact text content
//!   - `__pw_label=<text>` — find element by associated label text
//!   - `__pw_role=<role>` — find element by ARIA role
//!   - `__pw_role=<role>|<name>` — find element by ARIA role with accessible name

use pwright_cdp::CdpClient;
use pwright_cdp::connection::{CdpError, Result as CdpResult};

/// Extract the root nodeId from a `DOM.getDocument` response.
pub(super) fn root_node_id(doc: &serde_json::Value) -> CdpResult<i64> {
    doc.get("root")
        .and_then(|r| r.get("nodeId"))
        .and_then(|n| n.as_i64())
        .ok_or_else(|| CdpError::Other("DOM.getDocument missing root.nodeId".to_string()))
}

/// Resolved element — a nodeId from the DOM domain.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedElement {
    pub node_id: i64,
}

/// Resolve a selector to a single node, returning its nodeId.
///
/// Supports CSS selectors and special `__pw_*` prefixes.
pub async fn resolve_selector(
    session: &dyn CdpClient,
    selector: &str,
) -> CdpResult<Option<ResolvedElement>> {
    // __pw_nth=<base_selector>|<index>: resolve all CSS matches, then pick by index
    if let Some(rest) = selector.strip_prefix("__pw_nth=")
        && let Some(idx) = rest.rfind('|')
    {
        let base_selector = &rest[..idx];
        let index_str = &rest[idx + 1..];
        let index: i64 = index_str
            .parse()
            .map_err(|_| CdpError::Other(format!("invalid nth index: {index_str}")))?;
        // Use resolve_css_selector_all to avoid recursion
        let elements = resolve_css_selector_all(session, base_selector).await?;
        if elements.is_empty() {
            return Ok(None);
        }
        let resolved_index = if index < 0 {
            let positive = elements.len() as i64 + index;
            if positive < 0 {
                return Ok(None);
            }
            positive as usize
        } else {
            index as usize
        };
        return Ok(elements.get(resolved_index).copied());
    }

    resolve_pw_selector(session, selector).await
}

/// Resolve __pw_* prefixed selectors or plain CSS selectors (no __pw_nth).
async fn resolve_pw_selector(
    session: &dyn CdpClient,
    selector: &str,
) -> CdpResult<Option<ResolvedElement>> {
    if let Some(rest) = selector.strip_prefix("__pw_text_exact=") {
        return resolve_by_js(session, &js_find_by_text(rest, true)).await;
    }
    if let Some(rest) = selector.strip_prefix("__pw_text=") {
        return resolve_by_js(session, &js_find_by_text(rest, false)).await;
    }
    if let Some(rest) = selector.strip_prefix("__pw_label=") {
        return resolve_by_js(session, &js_find_by_label(rest)).await;
    }
    if let Some(rest) = selector.strip_prefix("__pw_role=") {
        return resolve_by_js(session, &js_find_by_role(rest)).await;
    }
    if let Some(rest) = selector.strip_prefix("__pw_filter_text=")
        && let Some(idx) = rest.rfind('|')
    {
        let base_selector = &rest[..idx];
        let text = &rest[idx + 1..];
        return resolve_by_js(session, &js_filter_by_text(base_selector, text)).await;
    }

    // Default: CSS selector
    let doc = session.dom_get_document().await?;
    let root_id = root_node_id(&doc)?;

    let node_id = session.dom_query_selector(root_id, selector).await?;
    if node_id == 0 {
        Ok(None)
    } else {
        Ok(Some(ResolvedElement { node_id }))
    }
}

/// Resolve a CSS selector to all matching nodes.
pub async fn resolve_selector_all(
    session: &dyn CdpClient,
    selector: &str,
) -> CdpResult<Vec<ResolvedElement>> {
    // For __pw_nth, resolve to the specific single element
    if selector.starts_with("__pw_nth=") {
        if let Some(elem) = resolve_selector(session, selector).await? {
            return Ok(vec![elem]);
        }
        return Ok(vec![]);
    }

    // For other __pw_* selectors, use JS-based resolution (single element only)
    if selector.starts_with("__pw_") {
        if let Some(elem) = resolve_pw_selector(session, selector).await? {
            return Ok(vec![elem]);
        }
        return Ok(vec![]);
    }

    resolve_css_selector_all(session, selector).await
}

/// Resolve a CSS selector to all matching nodes (no __pw_* handling).
async fn resolve_css_selector_all(
    session: &dyn CdpClient,
    selector: &str,
) -> CdpResult<Vec<ResolvedElement>> {
    let doc = session.dom_get_document().await?;
    let root_id = root_node_id(&doc)?;

    let node_ids = session.dom_query_selector_all(root_id, selector).await?;
    Ok(node_ids
        .into_iter()
        .map(|node_id| ResolvedElement { node_id })
        .collect())
}

/// Get an objectId for a resolved element (needed for callFunctionOn).
pub async fn resolve_object_id(session: &dyn CdpClient, node_id: i64) -> CdpResult<Option<String>> {
    // DOM.resolveNode works with nodeId (not just backendNodeId)
    let result = session.dom_resolve_node(node_id).await?;
    Ok(result
        .get("object")
        .and_then(|o| o.get("objectId"))
        .and_then(|id| id.as_str())
        .map(|s| s.to_string()))
}

// ── JS-based resolution helpers ──

/// Resolve an element using a JS expression that returns the element or null.
/// Uses Runtime.evaluate to find the element, then DOM.requestNode to get its nodeId.
async fn resolve_by_js(
    session: &dyn CdpClient,
    js_expr: &str,
) -> CdpResult<Option<ResolvedElement>> {
    // Must use returnByValue=false to get objectId for DOM elements
    let result = session.runtime_evaluate_as_object(js_expr).await?;

    // Check if the result is a DOM element (has objectId)
    let object_id = result
        .get("result")
        .and_then(|r| r.get("objectId"))
        .and_then(|id| id.as_str());

    let object_id = match object_id {
        Some(id) => id.to_string(),
        None => return Ok(None), // JS returned null/undefined or a primitive
    };

    // DOM.getDocument must be called first to enable the DOM domain,
    // otherwise DOM.requestNode fails with "No node with given id".
    session.dom_get_document().await?;

    // Use DOM.requestNode to convert the JS objectId → DOM nodeId
    let node_id = session.dom_request_node(&object_id).await?;
    if node_id == 0 {
        Ok(None)
    } else {
        Ok(Some(ResolvedElement { node_id }))
    }
}

/// Escape a value for safe use in CSS attribute selectors (`[attr="value"]`).
/// Prevents breakout via `\`, `"`, `]`, newlines, and null bytes.
pub fn css_escape_attr(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(']', "\\]")
        .replace('\n', "\\a ")
        .replace('\0', "\u{FFFD}")
}

/// Escape a string for safe embedding in JS. Uses serde_json which handles
/// newlines, backticks, null bytes, unicode separators, and all special chars.
/// Returns the string WITH surrounding quotes (e.g. `"hello \"world\""`)
fn js_escape(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Generate JS to find an element by text content.
fn js_find_by_text(text: &str, exact: bool) -> String {
    let escaped = js_escape(text);
    if exact {
        format!(
            "(() => {{ const t = {escaped}; const els = [...document.querySelectorAll('*')]; return els.find(el => el.childNodes.length > 0 && [...el.childNodes].some(n => n.nodeType === 3) && el.textContent.trim() === t) || null; }})()"
        )
    } else {
        format!(
            "(() => {{ const t = {escaped}; const els = [...document.querySelectorAll('*')]; return els.find(el => el.childNodes.length > 0 && [...el.childNodes].some(n => n.nodeType === 3 && n.textContent.includes(t)) ) || null; }})()"
        )
    }
}

/// Generate JS to filter elements by CSS selector and text content.
fn js_filter_by_text(base_selector: &str, text: &str) -> String {
    let escaped_text = js_escape(text);
    let escaped_selector = js_escape(base_selector);
    format!(
        "(() => {{ const sel = {escaped_selector}; const t = {escaped_text}; const els = [...document.querySelectorAll(sel)]; return els.find(el => el.textContent.includes(t)) || null; }})()"
    )
}

/// Generate JS to find an element by label text.
fn js_find_by_label(text: &str) -> String {
    let escaped = js_escape(text);
    format!(
        r#"(() => {{
  const t = {escaped};
  const labels = [...document.querySelectorAll('label')];
  for (const label of labels) {{
    if (label.textContent.trim().includes(t)) {{
      if (label.htmlFor) {{
        const target = document.getElementById(label.htmlFor);
        if (target) return target;
      }}
      const input = label.querySelector('input, textarea, select');
      if (input) return input;
    }}
  }}
  const ariaEl = document.querySelector('[aria-label=' + JSON.stringify(t) + ']');
  if (ariaEl) return ariaEl;
  return null;
}})()"#
    )
}

/// Generate JS to find an element by ARIA role.
/// Format: "role" or "role|name"
fn js_find_by_role(spec: &str) -> String {
    let (role, name) = if let Some(idx) = spec.find('|') {
        (&spec[..idx], Some(&spec[idx + 1..]))
    } else {
        (spec, None)
    };

    let css_escaped_role = css_escape_attr(role);

    // Map implicit roles to element selectors
    let implicit_selectors = match role {
        "button" => "button, [type=\"button\"], [type=\"submit\"], [type=\"reset\"]",
        "link" => "a[href]",
        "heading" => "h1, h2, h3, h4, h5, h6",
        "textbox" => {
            "input:not([type]), input[type=\"text\"], input[type=\"email\"], input[type=\"password\"], input[type=\"search\"], input[type=\"tel\"], input[type=\"url\"], textarea"
        }
        "checkbox" => "input[type=\"checkbox\"]",
        "radio" => "input[type=\"radio\"]",
        "img" => "img",
        "list" => "ul, ol",
        "listitem" => "li",
        "navigation" => "nav",
        "main" => "main",
        "banner" => "header",
        "contentinfo" => "footer",
        "complementary" => "aside",
        "form" => "form",
        "table" => "table",
        "row" => "tr",
        "cell" => "td",
        "columnheader" => "th",
        _ => "",
    };

    let name_filter = if let Some(n) = name {
        let escaped_name = js_escape(n);
        format!(
            r#"
  function matchesName(el) {{
    const n = {escaped_name};
    const ariaLabel = el.getAttribute('aria-label');
    if (ariaLabel && ariaLabel.includes(n)) return true;
    const labelledBy = el.getAttribute('aria-labelledby');
    if (labelledBy) {{
      const labelEl = document.getElementById(labelledBy);
      if (labelEl && labelEl.textContent.includes(n)) return true;
    }}
    if (el.textContent.trim().includes(n)) return true;
    if (el.title && el.title.includes(n)) return true;
    return false;
  }}"#
        )
    } else {
        "function matchesName() { return true; }".to_string()
    };

    format!(
        r#"(() => {{
  {name_filter}
  // 1. Explicit [role="..."]
  const explicit = [...document.querySelectorAll('[role="{css_escaped_role}"]')];
  for (const el of explicit) {{
    if (matchesName(el)) return el;
  }}
  // 2. Implicit role via element type
  const implicit = '{implicit_selectors}';
  if (implicit) {{
    const els = [...document.querySelectorAll(implicit)];
    for (const el of els) {{
      if (matchesName(el)) return el;
    }}
  }}
  return null;
}})()"#,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_resolve_selector_found() {
        let mock = MockCdpClient::new();
        mock.set_query_selector_response(42);
        let result = resolve_selector(&mock, "button.submit").await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().node_id, 42);
    }

    #[tokio::test]
    async fn test_resolve_selector_not_found() {
        let mock = MockCdpClient::new();
        // Default is 0 = not found
        let result = resolve_selector(&mock, ".nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_resolve_selector_all() {
        let mock = MockCdpClient::new();
        mock.set_query_selector_all_response(vec![10, 20, 30]);
        let results = resolve_selector_all(&mock, "li").await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].node_id, 10);
        assert_eq!(results[2].node_id, 30);
    }

    #[tokio::test]
    async fn test_resolve_selector_all_empty() {
        let mock = MockCdpClient::new();
        let results = resolve_selector_all(&mock, ".nothing").await.unwrap();
        assert!(results.is_empty());
    }
}
