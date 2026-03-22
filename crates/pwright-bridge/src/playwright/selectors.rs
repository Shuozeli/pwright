//! Selector parsing and resolution.
//!
//! Converts Playwright-style selectors into DOM operations.
//! Supports CSS selectors and typed semantic selectors via [`SelectorKind`].

use std::fmt;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::{CdpError, Result as CdpResult};

/// Extract the root nodeId from a `DOM.getDocument` response.
pub(super) fn root_node_id(doc: &serde_json::Value) -> CdpResult<i64> {
    doc.get("root")
        .and_then(|r| r.get("nodeId"))
        .and_then(|n| n.as_i64())
        .ok_or_else(|| CdpError::Other("DOM.getDocument missing root.nodeId".to_string()))
}

/// Typed selector — replaces the old `__pw_*` string-prefix encoding.
#[derive(Debug, Clone)]
pub enum SelectorKind {
    /// Plain CSS selector.
    Css(String),
    /// Find by text content (substring match).
    Text(String),
    /// Find by exact text content.
    TextExact(String),
    /// Find by label text (via `<label>` for/wrapping or `aria-label`).
    Label(String),
    /// Find by ARIA role, optionally filtered by accessible name.
    Role { role: String, name: Option<String> },
    /// Pick the nth match from a base selector.
    Nth { base: Box<SelectorKind>, index: i64 },
    /// Filter a base selector's matches by text content.
    FilterText {
        base: Box<SelectorKind>,
        text: String,
    },
}

impl fmt::Display for SelectorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Css(s) => write!(f, "{s}"),
            Self::Text(t) => write!(f, "text={t}"),
            Self::TextExact(t) => write!(f, "text_exact={t}"),
            Self::Label(t) => write!(f, "label={t}"),
            Self::Role { role, name: None } => write!(f, "role={role}"),
            Self::Role {
                role,
                name: Some(n),
            } => write!(f, "role={role}[{n}]"),
            Self::Nth { base, index } => write!(f, "{base}.nth({index})"),
            Self::FilterText { base, text } => write!(f, "{base}.filter_text({text})"),
        }
    }
}

/// Resolved element — a nodeId from the DOM domain.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedElement {
    pub node_id: i64,
}

/// Resolve a selector to a single node, returning its nodeId.
///
/// Uses `Box::pin` for recursive calls (Nth/FilterText on non-CSS bases)
/// because async recursion requires indirection to avoid infinite future sizes.
pub fn resolve_selector<'a>(
    session: &'a dyn CdpClient,
    selector: &'a SelectorKind,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = CdpResult<Option<ResolvedElement>>> + Send + 'a>,
> {
    Box::pin(async move {
        match selector {
            SelectorKind::Nth { base, index } => {
                let css_selector = match base.as_ref() {
                    SelectorKind::Css(s) => s.as_str(),
                    other => {
                        // Nth on non-CSS base: resolve the base, return single result
                        return resolve_selector(session, other).await;
                    }
                };
                let elements = resolve_css_selector_all(session, css_selector).await?;
                if elements.is_empty() {
                    return Ok(None);
                }
                let resolved_index = if *index < 0 {
                    let positive = elements.len() as i64 + index;
                    if positive < 0 {
                        return Ok(None);
                    }
                    positive as usize
                } else {
                    *index as usize
                };
                Ok(elements.get(resolved_index).copied())
            }
            SelectorKind::FilterText { base, text } => {
                let css_selector = match base.as_ref() {
                    SelectorKind::Css(s) => s.as_str(),
                    _ => return resolve_selector(session, base).await,
                };
                resolve_by_js(session, &js_filter_by_text(css_selector, text)).await
            }
            SelectorKind::TextExact(text) => {
                resolve_by_js(session, &js_find_by_text(text, true)).await
            }
            SelectorKind::Text(text) => resolve_by_js(session, &js_find_by_text(text, false)).await,
            SelectorKind::Label(text) => resolve_by_js(session, &js_find_by_label(text)).await,
            SelectorKind::Role { role, name } => {
                resolve_by_js(session, &js_find_by_role(role, name.as_deref())).await
            }
            SelectorKind::Css(css) => {
                let doc = session.dom_get_document().await?;
                let root_id = root_node_id(&doc)?;
                let node_id = session.dom_query_selector(root_id, css).await?;
                if node_id == 0 {
                    Ok(None)
                } else {
                    Ok(Some(ResolvedElement { node_id }))
                }
            }
        }
    })
}

/// Resolve a selector to all matching nodes.
pub async fn resolve_selector_all(
    session: &dyn CdpClient,
    selector: &SelectorKind,
) -> CdpResult<Vec<ResolvedElement>> {
    match selector {
        SelectorKind::Css(css) => resolve_css_selector_all(session, css).await,
        // Non-CSS selectors resolve to at most one element
        _ => {
            if let Some(elem) = resolve_selector(session, selector).await? {
                Ok(vec![elem])
            } else {
                Ok(vec![])
            }
        }
    }
}

/// Resolve a CSS selector to all matching nodes.
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

// ── JS-based resolution helpers ──

/// Resolve an element using a JS expression that returns the element or null.
async fn resolve_by_js(
    session: &dyn CdpClient,
    js_expr: &str,
) -> CdpResult<Option<ResolvedElement>> {
    let result = session.runtime_evaluate_as_object(js_expr).await?;

    let object_id = result
        .get("result")
        .and_then(|r| r.get("objectId"))
        .and_then(|id| id.as_str());

    let object_id = match object_id {
        Some(id) => id.to_string(),
        None => return Ok(None),
    };

    session.dom_get_document().await?;

    let node_id = session.dom_request_node(&object_id).await?;
    if node_id == 0 {
        Ok(None)
    } else {
        Ok(Some(ResolvedElement { node_id }))
    }
}

/// Escape a value for safe use in CSS attribute selectors (`[attr="value"]`).
pub fn css_escape_attr(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(']', "\\]")
        .replace('\n', "\\a ")
        .replace('\0', "\u{FFFD}")
}

/// Escape a string for safe embedding in JS.
fn js_escape(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

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

fn js_filter_by_text(base_selector: &str, text: &str) -> String {
    let escaped_text = js_escape(text);
    let escaped_selector = js_escape(base_selector);
    format!(
        "(() => {{ const sel = {escaped_selector}; const t = {escaped_text}; const els = [...document.querySelectorAll(sel)]; return els.find(el => el.textContent.includes(t)) || null; }})()"
    )
}

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

fn js_find_by_role(role: &str, name: Option<&str>) -> String {
    let css_escaped_role = css_escape_attr(role);

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
    async fn test_resolve_css_selector_found() {
        let mock = MockCdpClient::new();
        mock.set_query_selector_response(42);
        let sel = SelectorKind::Css("button.submit".into());
        let result = resolve_selector(&mock, &sel).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().node_id, 42);
    }

    #[tokio::test]
    async fn test_resolve_css_selector_not_found() {
        let mock = MockCdpClient::new();
        let sel = SelectorKind::Css(".nonexistent".into());
        let result = resolve_selector(&mock, &sel).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_resolve_selector_all() {
        let mock = MockCdpClient::new();
        mock.set_query_selector_all_response(vec![10, 20, 30]);
        let sel = SelectorKind::Css("li".into());
        let results = resolve_selector_all(&mock, &sel).await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].node_id, 10);
        assert_eq!(results[2].node_id, 30);
    }

    #[tokio::test]
    async fn test_resolve_selector_all_empty() {
        let mock = MockCdpClient::new();
        let sel = SelectorKind::Css(".nothing".into());
        let results = resolve_selector_all(&mock, &sel).await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn selector_kind_display() {
        assert_eq!(SelectorKind::Css("div".into()).to_string(), "div");
        assert_eq!(SelectorKind::Text("hello".into()).to_string(), "text=hello");
        assert_eq!(
            SelectorKind::Role {
                role: "button".into(),
                name: Some("Submit".into())
            }
            .to_string(),
            "role=button[Submit]"
        );
        let nth = SelectorKind::Nth {
            base: Box::new(SelectorKind::Css("li".into())),
            index: 2,
        };
        assert_eq!(nth.to_string(), "li.nth(2)");
    }
}
