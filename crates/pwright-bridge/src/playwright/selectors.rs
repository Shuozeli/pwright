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
use pwright_cdp::connection::Result as CdpResult;

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
    if let Some(rest) = selector.strip_prefix("__pw_filter_text=") {
        // Format: base_selector|text
        if let Some(idx) = rest.rfind('|') {
            let base_selector = &rest[..idx];
            let text = &rest[idx + 1..];
            return resolve_by_js(session, &js_filter_by_text(base_selector, text)).await;
        }
    }

    // Default: CSS selector
    let doc = session.dom_get_document().await?;
    let root_id = doc
        .get("root")
        .and_then(|r| r.get("nodeId"))
        .and_then(|n| n.as_i64())
        .unwrap_or(1);

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
    // For __pw_* selectors, we only support single resolution for now
    if selector.starts_with("__pw_") {
        if let Some(elem) = resolve_selector(session, selector).await? {
            return Ok(vec![elem]);
        }
        return Ok(vec![]);
    }

    let doc = session.dom_get_document().await?;
    let root_id = doc
        .get("root")
        .and_then(|r| r.get("nodeId"))
        .and_then(|n| n.as_i64())
        .unwrap_or(1);

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
/// Uses Runtime.evaluate to find the element, then DOM.describeNode to get its backendNodeId.
async fn resolve_by_js(
    session: &dyn CdpClient,
    js_expr: &str,
) -> CdpResult<Option<ResolvedElement>> {
    let result = session.runtime_evaluate(js_expr).await?;

    // Check if the result is a DOM element (has objectId)
    let object_id = result
        .get("result")
        .and_then(|r| r.get("objectId"))
        .and_then(|id| id.as_str());

    let object_id = match object_id {
        Some(id) => id.to_string(),
        None => return Ok(None), // JS returned null/undefined or a primitive
    };

    // Use DOM.describeNode to get the backendNodeId from the remote object
    // We can use the objectId to get node info via requesting the node
    let _node_result = session
        .runtime_call_function_on(&object_id, "function() { return this; }", vec![])
        .await?;

    // The result should have the node info. We need to get the backendNodeId.
    // Actually, let's use a different approach: evaluate JS that returns
    // a backendNodeId-like value we can use with DOM operations.

    // Use DOM.requestNode to convert objectId -> nodeId
    // But that's not in our trait. Let's use a workaround:
    // Call DOM.getDocument first, then use querySelectorAll and match.

    // Simpler: use Runtime.callFunctionOn to get an attribute we can use to locate via CSS.
    // Actually, let's just use a unique data attribute approach:
    let unique_id = format!(
        "__pw_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );

    // Tag the element with a unique attribute
    session
        .runtime_call_function_on(
            &object_id,
            &format!(
                "function() {{ this.setAttribute('data-pw-id', '{}'); }}",
                unique_id
            ),
            vec![],
        )
        .await?;

    // Now find it via CSS
    let doc = session.dom_get_document().await?;
    let root_id = doc
        .get("root")
        .and_then(|r| r.get("nodeId"))
        .and_then(|n| n.as_i64())
        .unwrap_or(1);

    let css = format!(r#"[data-pw-id="{}"]"#, unique_id);
    let node_id = session.dom_query_selector(root_id, &css).await?;

    // Clean up the temporary attribute
    let _ = session
        .runtime_call_function_on(
            &object_id,
            "function() { this.removeAttribute('data-pw-id'); }",
            vec![],
        )
        .await;

    if node_id == 0 {
        Ok(None)
    } else {
        Ok(Some(ResolvedElement { node_id }))
    }
}

/// Generate JS to find an element by text content.
fn js_find_by_text(text: &str, exact: bool) -> String {
    let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
    if exact {
        format!(
            "(() => {{ const els = [...document.querySelectorAll('*')]; return els.find(el => el.childNodes.length > 0 && [...el.childNodes].some(n => n.nodeType === 3) && el.textContent.trim() === '{}') || null; }})()",
            escaped
        )
    } else {
        format!(
            "(() => {{ const els = [...document.querySelectorAll('*')]; return els.find(el => el.childNodes.length > 0 && [...el.childNodes].some(n => n.nodeType === 3 && n.textContent.includes('{}')) ) || null; }})()",
            escaped
        )
    }
}

/// Generate JS to filter elements by CSS selector and text content.
fn js_filter_by_text(base_selector: &str, text: &str) -> String {
    let escaped_text = text.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_selector = base_selector.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        "(() => {{ const els = [...document.querySelectorAll('{}')]; return els.find(el => el.textContent.includes('{}')) || null; }})()",
        escaped_selector, escaped_text
    )
}

/// Generate JS to find an element by label text.
fn js_find_by_label(text: &str) -> String {
    let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
    format!(
        r#"(() => {{
  // 1. label[for] pointing to an input
  const labels = [...document.querySelectorAll('label')];
  for (const label of labels) {{
    if (label.textContent.trim().includes('{}')) {{
      if (label.htmlFor) {{
        const target = document.getElementById(label.htmlFor);
        if (target) return target;
      }}
      // 2. Wrapping label
      const input = label.querySelector('input, textarea, select');
      if (input) return input;
    }}
  }}
  // 3. aria-label
  const ariaEl = document.querySelector('[aria-label="{}"]');
  if (ariaEl) return ariaEl;
  return null;
}})()"#,
        escaped, escaped
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

    let escaped_role = role.replace('\\', "\\\\").replace('\'', "\\'");

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
        let escaped_name = n.replace('\\', "\\\\").replace('\'', "\\'");
        format!(
            r#"
  function matchesName(el) {{
    const ariaLabel = el.getAttribute('aria-label');
    if (ariaLabel && ariaLabel.includes('{}')) return true;
    const labelledBy = el.getAttribute('aria-labelledby');
    if (labelledBy) {{
      const labelEl = document.getElementById(labelledBy);
      if (labelEl && labelEl.textContent.includes('{}')) return true;
    }}
    if (el.textContent.trim().includes('{}')) return true;
    if (el.title && el.title.includes('{}')) return true;
    return false;
  }}"#,
            escaped_name, escaped_name, escaped_name, escaped_name
        )
    } else {
        "function matchesName() { return true; }".to_string()
    };

    format!(
        r#"(() => {{
  {name_filter}
  // 1. Explicit [role="..."]
  const explicit = [...document.querySelectorAll('[role="{escaped_role}"]')];
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
