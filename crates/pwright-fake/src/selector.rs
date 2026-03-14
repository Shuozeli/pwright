//! CSS selector matching against the in-memory DOM.
//!
//! Supports: tag, `.class`, `#id`, `\[attr\]`, `\[attr="val"\]`, descendant combinator,
//! and compound selectors (e.g. `button.submit\[disabled\]`).

use std::collections::HashMap;

use crate::dom::DomNode;

/// Match a CSS selector against a DOM tree, returning the first matching node ID.
pub fn query_selector(root: &DomNode, selector: &str) -> Option<i64> {
    query_selector_all(root, selector).into_iter().next()
}

/// Match a CSS selector against a DOM tree, returning all matching node IDs.
pub fn query_selector_all(root: &DomNode, selector: &str) -> Vec<i64> {
    let parts = parse_selector(selector);
    let all_nodes = root.all_nodes();
    let mut result = Vec::new();

    for node in &all_nodes {
        if node.tag == "#text" {
            continue;
        }
        if matches_selector_parts(node, &parts, &all_nodes) {
            result.push(node.node_id);
        }
    }
    result
}

/// A single simple selector (no combinators).
#[derive(Debug, Clone)]
struct SimpleSelector {
    tag: Option<String>,
    classes: Vec<String>,
    id: Option<String>,
    attrs: Vec<AttrSelector>,
}

#[derive(Debug, Clone)]
struct AttrSelector {
    name: String,
    value: Option<String>,
}

/// Parsed selector: chain of simple selectors separated by descendant combinators.
#[derive(Debug, Clone)]
struct SelectorParts {
    chain: Vec<SimpleSelector>,
}

fn parse_selector(selector: &str) -> SelectorParts {
    let segments = split_selector_segments(selector);
    let chain = segments.iter().map(|s| parse_simple_selector(s)).collect();
    SelectorParts { chain }
}

/// Split a selector string on whitespace, respecting quoted attribute values.
/// e.g. `div [placeholder="What needs to be done?"]` -> `["div", "[placeholder=...]"]`
fn split_selector_segments(selector: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_brackets = false;
    let mut in_quotes: Option<char> = None;

    for ch in selector.chars() {
        match in_quotes {
            Some(q) if ch == q => {
                in_quotes = None;
                current.push(ch);
            }
            Some(_) => {
                current.push(ch);
            }
            None => match ch {
                '"' | '\'' => {
                    in_quotes = Some(ch);
                    current.push(ch);
                }
                '[' => {
                    in_brackets = true;
                    current.push(ch);
                }
                ']' => {
                    in_brackets = false;
                    current.push(ch);
                }
                c if c.is_whitespace() && !in_brackets => {
                    if !current.is_empty() {
                        segments.push(current.clone());
                        current.clear();
                    }
                }
                _ => {
                    current.push(ch);
                }
            },
        }
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

fn parse_simple_selector(s: &str) -> SimpleSelector {
    let mut tag = None;
    let mut classes = Vec::new();
    let mut id = None;
    let mut attrs = Vec::new();

    let mut remaining = s;

    // Parse tag (everything before first . # or [)
    let tag_end = remaining
        .find(|c: char| ['.', '#', '['].contains(&c))
        .unwrap_or(remaining.len());
    if tag_end > 0 {
        let t = &remaining[..tag_end];
        if t != "*" {
            tag = Some(t.to_string());
        }
        remaining = &remaining[tag_end..];
    }

    while !remaining.is_empty() {
        if remaining.starts_with('.') {
            remaining = &remaining[1..];
            let end = remaining
                .find(|c: char| ['.', '#', '['].contains(&c))
                .unwrap_or(remaining.len());
            classes.push(remaining[..end].to_string());
            remaining = &remaining[end..];
        } else if remaining.starts_with('#') {
            remaining = &remaining[1..];
            let end = remaining
                .find(|c: char| ['.', '#', '['].contains(&c))
                .unwrap_or(remaining.len());
            id = Some(remaining[..end].to_string());
            remaining = &remaining[end..];
        } else if remaining.starts_with('[') {
            let close = remaining.find(']').unwrap_or(remaining.len());
            let inner = &remaining[1..close];
            if let Some(eq) = inner.find('=') {
                let attr_name = inner[..eq].to_string();
                let attr_val = inner[eq + 1..]
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                attrs.push(AttrSelector {
                    name: attr_name,
                    value: Some(attr_val),
                });
            } else {
                attrs.push(AttrSelector {
                    name: inner.to_string(),
                    value: None,
                });
            }
            remaining = remaining.get(close + 1..).unwrap_or("");
        } else {
            break;
        }
    }

    SimpleSelector {
        tag,
        classes,
        id,
        attrs,
    }
}

fn matches_simple(node: &DomNode, sel: &SimpleSelector) -> bool {
    if let Some(ref tag) = sel.tag
        && node.tag != *tag
    {
        return false;
    }
    if let Some(ref id) = sel.id
        && node.id() != Some(id.as_str())
    {
        return false;
    }
    let node_classes = node.classes();
    for class in &sel.classes {
        if !node_classes.contains(&class.as_str()) {
            return false;
        }
    }
    for attr in &sel.attrs {
        match &attr.value {
            Some(val) => {
                if node.attributes.get(&attr.name).map(|v| v.as_str()) != Some(val.as_str()) {
                    return false;
                }
            }
            None => {
                if !node.attributes.contains_key(&attr.name) {
                    return false;
                }
            }
        }
    }
    true
}

fn matches_selector_parts(node: &DomNode, parts: &SelectorParts, all_nodes: &[&DomNode]) -> bool {
    if parts.chain.is_empty() {
        return false;
    }

    let last = &parts.chain[parts.chain.len() - 1];
    if !matches_simple(node, last) {
        return false;
    }

    if parts.chain.len() == 1 {
        return true;
    }

    let ancestors = collect_ancestors(node.node_id, all_nodes);

    let mut ancestor_idx = 0;
    for i in (0..parts.chain.len() - 1).rev() {
        let sel = &parts.chain[i];
        let mut found = false;
        while ancestor_idx < ancestors.len() {
            if matches_simple(ancestors[ancestor_idx], sel) {
                ancestor_idx += 1;
                found = true;
                break;
            }
            ancestor_idx += 1;
        }
        if !found {
            return false;
        }
    }
    true
}

fn collect_ancestors<'a>(target_id: i64, all_nodes: &[&'a DomNode]) -> Vec<&'a DomNode> {
    let mut parent_map: HashMap<i64, i64> = HashMap::new();
    for node in all_nodes {
        for child in &node.children {
            build_parent_map(child, node.node_id, &mut parent_map);
        }
    }

    let mut ancestors = Vec::new();
    let mut current = target_id;
    while let Some(&parent_id) = parent_map.get(&current) {
        if let Some(parent) = all_nodes.iter().find(|n| n.node_id == parent_id) {
            ancestors.push(*parent);
        }
        current = parent_id;
    }
    ancestors
}

fn build_parent_map(node: &DomNode, parent_id: i64, map: &mut HashMap<i64, i64>) {
    map.insert(node.node_id, parent_id);
    for child in &node.children {
        build_parent_map(child, node.node_id, map);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tree() -> DomNode {
        DomNode {
            node_id: 1,
            tag: "div".to_string(),
            attributes: HashMap::from([("id".to_string(), "root".to_string())]),
            visible: true,
            value: None,
            text: None,
            children: vec![
                DomNode {
                    node_id: 2,
                    tag: "button".to_string(),
                    attributes: HashMap::from([
                        ("class".to_string(), "submit primary".to_string()),
                        ("disabled".to_string(), String::new()),
                    ]),
                    visible: true,
                    value: None,
                    text: Some("Submit".to_string()),
                    children: vec![],
                },
                DomNode {
                    node_id: 3,
                    tag: "input".to_string(),
                    attributes: HashMap::from([
                        ("type".to_string(), "checkbox".to_string()),
                        ("id".to_string(), "agree".to_string()),
                        ("checked".to_string(), String::new()),
                    ]),
                    visible: true,
                    value: None,
                    text: None,
                    children: vec![],
                },
                DomNode {
                    node_id: 4,
                    tag: "ul".to_string(),
                    attributes: HashMap::new(),
                    visible: true,
                    value: None,
                    text: None,
                    children: vec![
                        DomNode {
                            node_id: 5,
                            tag: "li".to_string(),
                            attributes: HashMap::from([("class".to_string(), "item".to_string())]),
                            visible: true,
                            value: None,
                            text: Some("Item 1".to_string()),
                            children: vec![],
                        },
                        DomNode {
                            node_id: 6,
                            tag: "li".to_string(),
                            attributes: HashMap::from([(
                                "class".to_string(),
                                "item active".to_string(),
                            )]),
                            visible: true,
                            value: None,
                            text: Some("Item 2".to_string()),
                            children: vec![],
                        },
                        DomNode {
                            node_id: 7,
                            tag: "li".to_string(),
                            attributes: HashMap::from([("class".to_string(), "item".to_string())]),
                            visible: true,
                            value: None,
                            text: Some("Item 3".to_string()),
                            children: vec![],
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn tag_selector() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, "button"), Some(2));
        assert_eq!(query_selector_all(&tree, "li").len(), 3);
    }

    #[test]
    fn class_selector() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, ".submit"), Some(2));
        assert_eq!(query_selector(&tree, ".active"), Some(6));
        assert_eq!(query_selector(&tree, ".nonexistent"), None);
    }

    #[test]
    fn id_selector() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, "#agree"), Some(3));
        assert_eq!(query_selector(&tree, "#root"), Some(1));
    }

    #[test]
    fn attribute_presence() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, "[disabled]"), Some(2));
        assert_eq!(query_selector(&tree, "[checked]"), Some(3));
    }

    #[test]
    fn attribute_value() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, r#"[type="checkbox"]"#), Some(3));
    }

    #[test]
    fn compound_selector() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, "button.submit"), Some(2));
        assert_eq!(query_selector(&tree, "li.active"), Some(6));
        assert_eq!(query_selector(&tree, "li.submit"), None);
    }

    #[test]
    fn descendant_selector() {
        let tree = make_tree();
        assert_eq!(query_selector_all(&tree, "ul li"), vec![5, 6, 7]);
        assert_eq!(query_selector(&tree, "div button"), Some(2));
        assert_eq!(query_selector(&tree, "div ul li.active"), Some(6));
    }

    #[test]
    fn compound_with_attribute() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, r#"input[type="checkbox"]"#), Some(3));
        assert_eq!(query_selector(&tree, "button[disabled]"), Some(2));
    }

    #[test]
    fn no_match() {
        let tree = make_tree();
        assert_eq!(query_selector(&tree, "span"), None);
        assert!(query_selector_all(&tree, "table").is_empty());
    }
}
