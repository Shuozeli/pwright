//! Simple HTML parser that builds a `DomNode` tree.
//!
//! Handles the subset of HTML used in test fixtures: tags with attributes,
//! text content, self-closing tags, and nesting. Not a full HTML5 parser.

use std::collections::HashMap;

use crate::dom::{DomNode, NodeIdGen};

/// Parse an HTML string into a `DomNode` tree.
///
/// Wraps the content in a synthetic `<html>` root if it doesn't start with one.
pub fn parse_html(html: &str) -> DomNode {
    let id_gen = NodeIdGen::new();
    let trimmed = html.trim();

    // If the content is a fragment (no <html> root), wrap it
    let wrapped = if trimmed.starts_with("<html") || trimmed.starts_with("<!") {
        trimmed.to_string()
    } else {
        format!("<body>{trimmed}</body>")
    };

    let mut parser = Parser {
        input: wrapped.as_str(),
        pos: 0,
        id_gen,
    };
    parser.parse_node().unwrap_or_else(|| DomNode {
        node_id: 0,
        tag: "body".to_string(),
        attributes: HashMap::new(),
        children: vec![],
        text: None,
        visible: true,
        value: None,
    })
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    id_gen: NodeIdGen,
}

const VOID_ELEMENTS: &[&str] = &[
    "input", "br", "hr", "img", "meta", "link", "area", "base", "col",
];

impl<'a> Parser<'a> {
    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn parse_node(&mut self) -> Option<DomNode> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return None;
        }
        if self.remaining().starts_with("<!--") {
            // Skip comments
            if let Some(end) = self.remaining().find("-->") {
                self.pos += end + 3;
                return self.parse_node();
            }
        }
        if self.remaining().starts_with("<!") {
            // Skip doctype
            if let Some(end) = self.remaining().find('>') {
                self.pos += end + 1;
                return self.parse_node();
            }
        }
        if self.remaining().starts_with('<') && !self.remaining().starts_with("</") {
            self.parse_element()
        } else if !self.remaining().starts_with("</") {
            self.parse_text()
        } else {
            None
        }
    }

    fn parse_element(&mut self) -> Option<DomNode> {
        // Skip '<'
        self.pos += 1;

        let tag = self.parse_tag_name()?;
        let attributes = self.parse_attributes();

        // Check for self-closing /> or void element
        self.skip_whitespace();
        let self_closing = self.remaining().starts_with("/>");
        if self_closing {
            self.pos += 2;
        } else if self.remaining().starts_with('>') {
            self.pos += 1;
        }

        let is_void = VOID_ELEMENTS.contains(&tag.as_str());

        let children = if self_closing || is_void {
            vec![]
        } else {
            self.parse_children(&tag)
        };

        let value = if matches!(tag.as_str(), "input" | "textarea" | "select") {
            attributes.get("value").cloned()
        } else {
            None
        };

        Some(DomNode {
            node_id: self.id_gen.next(),
            tag,
            attributes,
            children,
            text: None,
            visible: true,
            value,
        })
    }

    fn parse_tag_name(&mut self) -> Option<String> {
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        Some(self.input[start..self.pos].to_lowercase())
    }

    fn parse_attributes(&mut self) -> HashMap<String, String> {
        let mut attrs = HashMap::new();
        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len()
                || self.remaining().starts_with('>')
                || self.remaining().starts_with("/>")
            {
                break;
            }
            if let Some((name, value)) = self.parse_attribute() {
                attrs.insert(name, value);
            } else {
                break;
            }
        }
        attrs
    }

    fn parse_attribute(&mut self) -> Option<(String, String)> {
        let name_start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_' || ch == b':' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == name_start {
            return None;
        }
        let name = self.input[name_start..self.pos].to_lowercase();

        self.skip_whitespace();
        if !self.remaining().starts_with('=') {
            // Boolean attribute (e.g. `checked`, `disabled`)
            return Some((name, String::new()));
        }
        self.pos += 1; // skip '='
        self.skip_whitespace();

        let value = if self.remaining().starts_with('"') {
            self.pos += 1;
            let end = self.remaining().find('"').unwrap_or(self.remaining().len());
            let val = self.input[self.pos..self.pos + end].to_string();
            self.pos += end + 1;
            val
        } else if self.remaining().starts_with('\'') {
            self.pos += 1;
            let end = self
                .remaining()
                .find('\'')
                .unwrap_or(self.remaining().len());
            let val = self.input[self.pos..self.pos + end].to_string();
            self.pos += end + 1;
            val
        } else {
            // Unquoted value
            let end = self
                .remaining()
                .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
                .unwrap_or(self.remaining().len());
            let val = self.input[self.pos..self.pos + end].to_string();
            self.pos += end;
            val
        };

        Some((name, value))
    }

    fn parse_children(&mut self, parent_tag: &str) -> Vec<DomNode> {
        let mut children = Vec::new();
        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len() {
                break;
            }
            // Check for closing tag
            let closing = format!("</{parent_tag}>");
            if self.remaining().to_lowercase().starts_with(&closing) {
                self.pos += closing.len();
                break;
            }
            // Also handle closing tag with spaces: </tag >
            if self.remaining().starts_with("</") {
                // Some other closing tag or malformed - stop
                if let Some(end) = self.remaining().find('>') {
                    self.pos += end + 1;
                }
                break;
            }
            if let Some(child) = self.parse_node() {
                children.push(child);
            } else {
                // Avoid infinite loop
                if self.pos < self.input.len() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        children
    }

    fn parse_text(&mut self) -> Option<DomNode> {
        let start = self.pos;
        while self.pos < self.input.len() && !self.remaining().starts_with('<') {
            self.pos += 1;
        }
        let text = self.input[start..self.pos].to_string();
        // Skip whitespace-only text nodes (indentation), preserve content otherwise
        if text.trim().is_empty() {
            return None;
        }
        Some(DomNode {
            node_id: self.id_gen.next(),
            tag: "#text".to_string(),
            attributes: HashMap::new(),
            children: vec![],
            text: Some(text),
            visible: true,
            value: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selector;

    #[test]
    fn parse_simple_div() {
        let tree = parse_html("<div>hello</div>");
        // Wrapped in <body>
        assert_eq!(tree.tag, "body");
        let div = &tree.children[0];
        assert_eq!(div.tag, "div");
        assert_eq!(div.text_content(), "hello");
    }

    #[test]
    fn parse_attributes() {
        let tree = parse_html(r#"<input type="checkbox" id="agree" checked />"#);
        let input = &tree.children[0];
        assert_eq!(input.tag, "input");
        assert_eq!(input.attributes.get("type").unwrap(), "checkbox");
        assert_eq!(input.attributes.get("id").unwrap(), "agree");
        assert!(input.attributes.contains_key("checked"));
    }

    #[test]
    fn parse_nested() {
        let tree = parse_html(
            r#"
            <div id="app">
                <h1>Welcome</h1>
                <ul>
                    <li class="item">Item 1</li>
                    <li class="item active">Item 2</li>
                </ul>
            </div>
        "#,
        );

        // Should be queryable
        assert!(selector::query_selector(&tree, "#app").is_some());
        assert!(selector::query_selector(&tree, "h1").is_some());
        assert_eq!(selector::query_selector_all(&tree, "li").len(), 2);
        assert!(selector::query_selector(&tree, "li.active").is_some());
    }

    #[test]
    fn parse_void_elements() {
        let tree = parse_html(r#"<div><input type="text" /><br><hr></div>"#);
        let div = &tree.children[0];
        assert_eq!(div.children.len(), 3);
        assert_eq!(div.children[0].tag, "input");
        assert_eq!(div.children[1].tag, "br");
        assert_eq!(div.children[2].tag, "hr");
    }

    #[test]
    fn parse_full_page() {
        let tree = parse_html(
            r#"
            <div id="app">
                <h1>Welcome</h1>
                <input type="checkbox" id="agree" checked />
                <button class="submit" disabled>Submit</button>
                <ul>
                    <li>Item 1</li>
                    <li>Item 2</li>
                    <li>Item 3</li>
                </ul>
            </div>
        "#,
        );

        // Verify full selector coverage
        assert!(selector::query_selector(&tree, "button.submit").is_some());
        assert!(selector::query_selector(&tree, "[disabled]").is_some());
        assert!(selector::query_selector(&tree, "#agree").is_some());
        assert_eq!(selector::query_selector_all(&tree, "ul li").len(), 3);

        // Verify text content
        let h1_id = selector::query_selector(&tree, "h1").unwrap();
        let h1 = tree.find_by_node_id(h1_id).unwrap();
        assert_eq!(h1.text_content(), "Welcome");

        // Verify checkbox property
        let input_id = selector::query_selector(&tree, r#"input[type="checkbox"]"#).unwrap();
        let input = tree.find_by_node_id(input_id).unwrap();
        assert!(input.has_property("checked"));
    }

    #[test]
    fn parse_placeholder_with_spaces_in_value() {
        let tree = parse_html(r#"<input class="new-todo" placeholder="What needs to be done?" />"#);
        let result = selector::query_selector(&tree, r#"[placeholder="What needs to be done?"]"#);
        assert!(result.is_some());
    }

    #[test]
    fn parse_data_attributes() {
        let tree = parse_html(r#"<button data-testid="login-btn">Login</button>"#);
        assert!(selector::query_selector(&tree, r#"[data-testid="login-btn"]"#).is_some());
    }
}
