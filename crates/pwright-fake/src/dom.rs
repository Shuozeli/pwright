//! In-memory DOM tree for FakeCdpClient.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};

/// A node in the in-memory DOM tree.
#[derive(Debug, Clone)]
pub struct DomNode {
    pub node_id: i64,
    pub tag: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<DomNode>,
    pub text: Option<String>,
    /// Whether the node has layout (visible). Default: true.
    pub visible: bool,
    /// Input value (for `<input>`, `<textarea>`, `<select>`).
    pub value: Option<String>,
}

impl DomNode {
    /// Get all text content recursively (like `textContent` in JS).
    pub fn text_content(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    fn collect_text(&self, out: &mut String) {
        if let Some(ref text) = self.text {
            out.push_str(text);
        }
        for child in &self.children {
            child.collect_text(out);
        }
    }

    /// Get inner HTML (serialized children).
    pub fn inner_html(&self) -> String {
        let mut out = String::new();
        for child in &self.children {
            child.serialize(&mut out);
        }
        out
    }

    /// Get outer HTML (serialized self including tag).
    pub fn outer_html(&self) -> String {
        let mut out = String::new();
        self.serialize(&mut out);
        out
    }

    /// Void elements that don't get a closing tag.
    fn is_void_element(tag: &str) -> bool {
        matches!(
            tag,
            "input" | "br" | "hr" | "img" | "meta" | "link" | "area" | "base" | "col"
        )
    }

    fn serialize(&self, out: &mut String) {
        if self.tag == "#text" {
            if let Some(ref text) = self.text {
                out.push_str(text);
            }
            return;
        }
        out.push('<');
        out.push_str(&self.tag);
        // Sort attributes for deterministic output
        let mut attrs: Vec<_> = self.attributes.iter().collect();
        attrs.sort_by_key(|(k1, _)| *k1);
        for (key, val) in &attrs {
            out.push(' ');
            out.push_str(key);
            if !val.is_empty() {
                out.push_str("=\"");
                out.push_str(val);
                out.push('"');
            }
        }
        if Self::is_void_element(&self.tag) {
            out.push_str(" />");
            return;
        }
        out.push('>');
        for child in &self.children {
            child.serialize(out);
        }
        out.push_str("</");
        out.push_str(&self.tag);
        out.push('>');
    }

    /// Get attributes as a flat [name, value, name, value, ...] vec (CDP format).
    pub fn attributes_flat(&self) -> Vec<String> {
        let mut result = Vec::new();
        for (key, val) in &self.attributes {
            result.push(key.clone());
            result.push(val.clone());
        }
        result
    }

    /// Check if a DOM property is true (checked, disabled).
    pub fn has_property(&self, name: &str) -> bool {
        match name {
            "checked" => self.attributes.contains_key("checked"),
            "disabled" => self.attributes.contains_key("disabled"),
            _ => false,
        }
    }

    /// Get CSS classes.
    pub fn classes(&self) -> Vec<&str> {
        self.attributes
            .get("class")
            .map(|c| c.split_whitespace().collect())
            .unwrap_or_default()
    }

    /// Get the ID attribute.
    pub fn id(&self) -> Option<&str> {
        self.attributes.get("id").map(|s| s.as_str())
    }

    /// Find a node by ID in the subtree.
    pub fn find_by_node_id(&self, target: i64) -> Option<&DomNode> {
        if self.node_id == target {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_node_id(target) {
                return Some(found);
            }
        }
        None
    }

    /// Find a mutable node by ID in the subtree.
    pub fn find_by_node_id_mut(&mut self, target: i64) -> Option<&mut DomNode> {
        if self.node_id == target {
            return Some(self);
        }
        for child in &mut self.children {
            if let Some(found) = child.find_by_node_id_mut(target) {
                return Some(found);
            }
        }
        None
    }

    /// Collect all nodes in the subtree as a flat list.
    pub fn all_nodes(&self) -> Vec<&DomNode> {
        let mut result = vec![self];
        for child in &self.children {
            result.extend(child.all_nodes());
        }
        result
    }
}

/// Node ID allocator.
pub struct NodeIdGen {
    next: AtomicI64,
}

impl NodeIdGen {
    pub fn new() -> Self {
        Self {
            next: AtomicI64::new(1),
        }
    }

    pub fn next(&self) -> i64 {
        self.next.fetch_add(1, Ordering::Relaxed)
    }
}

impl Default for NodeIdGen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_node() -> DomNode {
        DomNode {
            node_id: 1,
            tag: "div".to_string(),
            attributes: HashMap::from([("class".to_string(), "container".to_string())]),
            visible: true,
            value: None,
            text: None,
            children: vec![
                DomNode {
                    node_id: 2,
                    tag: "#text".to_string(),
                    attributes: HashMap::new(),
                    children: vec![],
                    text: Some("Hello ".to_string()),
                    visible: true,
                    value: None,
                },
                DomNode {
                    node_id: 3,
                    tag: "span".to_string(),
                    attributes: HashMap::new(),
                    children: vec![DomNode {
                        node_id: 4,
                        tag: "#text".to_string(),
                        attributes: HashMap::new(),
                        children: vec![],
                        text: Some("World".to_string()),
                        visible: true,
                        value: None,
                    }],
                    text: None,
                    visible: true,
                    value: None,
                },
            ],
        }
    }

    #[test]
    fn text_content_recursive() {
        assert_eq!(sample_node().text_content(), "Hello World");
    }

    #[test]
    fn find_by_node_id_found() {
        let node = sample_node();
        let found = node.find_by_node_id(3).unwrap();
        assert_eq!(found.tag, "span");
    }

    #[test]
    fn find_by_node_id_not_found() {
        assert!(sample_node().find_by_node_id(99).is_none());
    }

    #[test]
    fn classes_parsed() {
        assert_eq!(sample_node().classes(), vec!["container"]);
    }

    #[test]
    fn has_property_checked() {
        let mut node = sample_node();
        assert!(!node.has_property("checked"));
        node.attributes.insert("checked".to_string(), String::new());
        assert!(node.has_property("checked"));
    }

    #[test]
    fn all_nodes_count() {
        assert_eq!(sample_node().all_nodes().len(), 4);
    }
}
