//! Accessibility tree snapshot — builds a flat list from CDP's AX tree.
//! Mirrors PinchTab's snapshot.go: BuildSnapshot, InteractiveRoles, ref cache.

use std::collections::BTreeMap;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use pwright_cdp::domains::accessibility::RawAXNode;

/// Processed accessibility node.
#[derive(Debug, Clone)]
pub struct A11yNode {
    pub ref_id: String,
    pub role: String,
    pub name: String,
    pub depth: i32,
    pub value: String,
    pub disabled: bool,
    pub focused: bool,
    pub node_id: i64,
}

/// Ref cache mapping element refs to backend DOM node IDs.
#[derive(Debug, Clone, Default)]
pub struct RefCache {
    pub refs: BTreeMap<String, i64>,
    pub nodes: Vec<A11yNode>,
}

/// Snapshot filter mode.
#[derive(Debug, Clone, Default)]
pub enum SnapshotFilter {
    #[default]
    All,
    Interactive,
}

/// Roles considered interactive (matching PinchTab's InteractiveRoles).
fn is_interactive_role(role: &str) -> bool {
    matches!(
        role,
        "button"
            | "link"
            | "textbox"
            | "searchbox"
            | "combobox"
            | "listbox"
            | "option"
            | "checkbox"
            | "radio"
            | "switch"
            | "slider"
            | "spinbutton"
            | "menuitem"
            | "menuitemcheckbox"
            | "menuitemradio"
            | "tab"
            | "treeitem"
    )
}

/// Build a snapshot from raw AX nodes.
///
/// Mirrors PinchTab's BuildSnapshot algorithm:
/// 1. Build parent map to compute depths
/// 2. Filter nodes by role/filter
/// 3. Assign refs (e0, e1, ...) and track backendDOMNodeId
pub fn build_snapshot(
    nodes: &[RawAXNode],
    filter: &SnapshotFilter,
    max_depth: i32,
) -> (Vec<A11yNode>, BTreeMap<String, i64>) {
    let mut parent_map: BTreeMap<&str, &str> = BTreeMap::new();
    for n in nodes {
        for child_id in &n.child_ids {
            parent_map.insert(child_id.as_str(), n.node_id.as_str());
        }
    }

    let depth_of = |node_id: &str| -> i32 {
        let mut d = 0i32;
        let mut cur = node_id;
        while let Some(parent) = parent_map.get(cur) {
            d += 1;
            cur = parent;
        }
        d
    };

    let mut flat = Vec::new();
    let mut refs: BTreeMap<String, i64> = BTreeMap::new();
    let mut ref_id = 0;

    for n in nodes {
        if n.ignored {
            continue;
        }

        let role = n.role.as_ref().map(|v| v.as_str()).unwrap_or_default();
        let name = n.name.as_ref().map(|v| v.as_str()).unwrap_or_default();

        if role == "none" || role == "InlineTextBox" {
            continue;
        }
        if name.is_empty() && role == "StaticText" {
            continue;
        }

        let depth = depth_of(&n.node_id);
        if max_depth >= 0 && depth > max_depth {
            continue;
        }

        if matches!(filter, SnapshotFilter::Interactive) && !is_interactive_role(role) {
            continue;
        }

        let ref_str = format!("e{}", ref_id);

        let mut entry = A11yNode {
            ref_id: ref_str.clone(),
            role: role.to_string(),
            name: name.to_string(),
            depth,
            value: n
                .value
                .as_ref()
                .map(|v| v.as_str().to_string())
                .unwrap_or_default(),
            disabled: false,
            focused: false,
            node_id: n.backend_dom_node_id,
        };

        for prop in &n.properties {
            if prop.name == "disabled" && prop.value.as_str() == "true" {
                entry.disabled = true;
            }
            if prop.name == "focused" && prop.value.as_str() == "true" {
                entry.focused = true;
            }
        }

        if n.backend_dom_node_id != 0 {
            refs.insert(ref_str, n.backend_dom_node_id);
        }

        flat.push(entry);
        ref_id += 1;
    }

    (flat, refs)
}

/// Get a full snapshot for a tab via CDP.
pub async fn get_snapshot(
    session: &dyn CdpClient,
    filter: &SnapshotFilter,
    max_depth: i32,
) -> CdpResult<(Vec<A11yNode>, BTreeMap<String, i64>)> {
    // Accessibility domain must be enabled to get a full tree
    session.accessibility_enable().await?;
    let raw_nodes = session.accessibility_get_full_tree().await?;
    tracing::trace!(raw_nodes = raw_nodes.len(), "snapshot: CDP AX tree fetched");
    Ok(build_snapshot(&raw_nodes, filter, max_depth))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_cdp::domains::accessibility::{AXProperty, AXValue};
    use serde_json::Value;

    fn ax_value(s: &str) -> AXValue {
        AXValue {
            value_type: "string".to_string(),
            value: Value::String(s.to_string()),
        }
    }

    fn make_node(
        node_id: &str,
        role: &str,
        name: &str,
        backend_id: i64,
        children: Vec<&str>,
    ) -> RawAXNode {
        RawAXNode {
            node_id: node_id.to_string(),
            ignored: false,
            role: if role.is_empty() {
                None
            } else {
                Some(ax_value(role))
            },
            name: if name.is_empty() {
                None
            } else {
                Some(ax_value(name))
            },
            value: None,
            properties: vec![],
            child_ids: children.iter().map(|s| s.to_string()).collect(),
            backend_dom_node_id: backend_id,
        }
    }

    #[test]
    fn test_build_snapshot_basic() {
        let nodes = vec![
            make_node("1", "document", "page", 0, vec!["2", "3"]),
            make_node("2", "heading", "Welcome", 10, vec![]),
            make_node("3", "button", "Submit", 20, vec![]),
        ];

        let (flat, refs) = build_snapshot(&nodes, &SnapshotFilter::All, -1);

        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].role, "document");
        assert_eq!(flat[1].role, "heading");
        assert_eq!(flat[2].role, "button");
        assert_eq!(flat[2].name, "Submit");

        // Depth: document=0, heading=1, button=1
        assert_eq!(flat[0].depth, 0);
        assert_eq!(flat[1].depth, 1);
        assert_eq!(flat[2].depth, 1);

        // Refs
        assert_eq!(refs.get("e1"), Some(&10));
        assert_eq!(refs.get("e2"), Some(&20));
    }

    #[test]
    fn test_build_snapshot_interactive_filter() {
        let nodes = vec![
            make_node("1", "document", "page", 0, vec!["2", "3", "4"]),
            make_node("2", "heading", "Welcome", 10, vec![]),
            make_node("3", "button", "Submit", 20, vec![]),
            make_node("4", "textbox", "Name", 30, vec![]),
        ];

        let (flat, refs) = build_snapshot(&nodes, &SnapshotFilter::Interactive, -1);

        // Only button and textbox are interactive
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].role, "button");
        assert_eq!(flat[1].role, "textbox");
        assert_eq!(refs.get("e0"), Some(&20));
        assert_eq!(refs.get("e1"), Some(&30));
    }

    #[test]
    fn test_build_snapshot_skips_ignored() {
        let mut nodes = vec![make_node("1", "button", "OK", 10, vec![])];
        nodes[0].ignored = true;

        let (flat, _) = build_snapshot(&nodes, &SnapshotFilter::All, -1);
        assert!(flat.is_empty());
    }

    #[test]
    fn test_build_snapshot_skips_none_roles() {
        let nodes = vec![
            make_node("1", "none", "", 10, vec![]),
            make_node("2", "generic", "container", 20, vec![]),
            make_node("3", "InlineTextBox", "", 30, vec![]),
            make_node("4", "button", "OK", 40, vec![]),
        ];

        let (flat, _) = build_snapshot(&nodes, &SnapshotFilter::All, -1);
        // generic is kept; none and InlineTextBox are filtered
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].role, "generic");
        assert_eq!(flat[1].role, "button");
    }

    #[test]
    fn test_build_snapshot_skips_empty_static_text() {
        let nodes = vec![
            make_node("1", "StaticText", "", 10, vec![]), // empty name → skip
            make_node("2", "StaticText", "Hello", 20, vec![]), // has name → keep
        ];

        let (flat, _) = build_snapshot(&nodes, &SnapshotFilter::All, -1);
        assert_eq!(flat.len(), 1);
        assert_eq!(flat[0].name, "Hello");
    }

    #[test]
    fn test_build_snapshot_max_depth() {
        let nodes = vec![
            make_node("1", "document", "page", 0, vec!["2"]),
            make_node("2", "region", "main", 10, vec!["3"]),
            make_node("3", "button", "Deep", 20, vec![]),
        ];

        // max_depth=1 → document(0) + region(1), skip button(2)
        let (flat, _) = build_snapshot(&nodes, &SnapshotFilter::All, 1);
        assert_eq!(flat.len(), 2);
        assert_eq!(flat[0].role, "document");
        assert_eq!(flat[1].role, "region");
    }

    #[test]
    fn test_build_snapshot_properties_disabled_focused() {
        let mut node = make_node("1", "button", "OK", 10, vec![]);
        node.properties = vec![
            AXProperty {
                name: "disabled".to_string(),
                value: ax_value("true"),
            },
            AXProperty {
                name: "focused".to_string(),
                value: ax_value("true"),
            },
        ];

        let (flat, _) = build_snapshot(&[node], &SnapshotFilter::All, -1);
        assert!(flat[0].disabled);
        assert!(flat[0].focused);
    }

    #[test]
    fn test_build_snapshot_no_ref_for_zero_backend_id() {
        let nodes = vec![make_node("1", "document", "page", 0, vec![])];
        let (flat, refs) = build_snapshot(&nodes, &SnapshotFilter::All, -1);
        assert_eq!(flat.len(), 1);
        assert!(refs.is_empty()); // backendDOMNodeId=0 → no ref
    }

    #[test]
    fn test_ref_numbering_sequential() {
        let nodes = vec![
            make_node("1", "button", "A", 10, vec![]),
            make_node("2", "link", "B", 20, vec![]),
            make_node("3", "textbox", "C", 30, vec![]),
        ];

        let (flat, _) = build_snapshot(&nodes, &SnapshotFilter::All, -1);
        assert_eq!(flat[0].ref_id, "e0");
        assert_eq!(flat[1].ref_id, "e1");
        assert_eq!(flat[2].ref_id, "e2");
    }
}
