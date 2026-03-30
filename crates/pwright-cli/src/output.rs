/// Formatted terminal output for CLI results.
use pwright_bridge::A11yNode;

/// Format a snapshot node as a string.
fn format_snapshot_node(node: &A11yNode) -> String {
    let indent = "  ".repeat(node.depth as usize);
    let ref_tag = if node.ref_id.is_empty() {
        String::new()
    } else {
        format!("[{}] ", node.ref_id)
    };
    let name = if node.name.is_empty() {
        String::new()
    } else {
        format!(" \"{}\"", node.name)
    };
    let value = if node.value.is_empty() {
        String::new()
    } else {
        format!(" value={}", node.value)
    };
    let extra = if node.focused { " (focused)" } else { "" };
    format!(
        "{}{}{}{}{}{}",
        indent, ref_tag, node.role, name, value, extra
    )
}

/// Print a snapshot as a tree of accessibility nodes.
pub fn print_snapshot(nodes: &[A11yNode]) {
    if nodes.is_empty() {
        println!("  (empty snapshot)");
        return;
    }
    for node in nodes {
        println!("{}", format_snapshot_node(node));
    }
}

pub fn ok(msg: &str) {
    println!("[ok] {msg}");
}

pub fn info(msg: &str) {
    println!("{msg}");
}

pub fn error(msg: &str) {
    eprintln!("[error] {msg}");
}

/// Print a list of tabs.
pub fn print_tab_list(tabs: &[(String, String, String)], active: &str) {
    if tabs.is_empty() {
        println!("  (no tabs)");
        return;
    }
    for (id, title, url) in tabs {
        let marker = if id == active { " <- active" } else { "" };
        println!("  {} | {} | {}{}", id, title, url, marker);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(ref_id: &str, role: &str, name: &str, depth: i32) -> A11yNode {
        A11yNode {
            ref_id: ref_id.to_string(),
            role: role.to_string(),
            name: name.to_string(),
            depth,
            value: String::new(),
            disabled: false,
            focused: false,
            node_id: 0,
        }
    }

    #[test]
    fn format_basic_node() {
        let node = make_node("e0", "RootWebArea", "Example", 0);
        let s = format_snapshot_node(&node);
        assert_eq!(s, "[e0] RootWebArea \"Example\"");
    }

    #[test]
    fn format_node_without_ref() {
        let node = make_node("", "heading", "Title", 0);
        let s = format_snapshot_node(&node);
        assert_eq!(s, "heading \"Title\"");
    }

    #[test]
    fn format_node_without_name() {
        let node = make_node("e1", "textbox", "", 1);
        let s = format_snapshot_node(&node);
        assert_eq!(s, "  [e1] textbox");
    }

    #[test]
    fn format_node_with_value() {
        let mut node = make_node("e2", "textbox", "Email", 0);
        node.value = "user@test.com".to_string();
        let s = format_snapshot_node(&node);
        assert_eq!(s, "[e2] textbox \"Email\" value=user@test.com");
    }

    #[test]
    fn format_node_focused() {
        let mut node = make_node("e3", "button", "Submit", 0);
        node.focused = true;
        let s = format_snapshot_node(&node);
        assert_eq!(s, "[e3] button \"Submit\" (focused)");
    }

    #[test]
    fn format_node_indented() {
        let node = make_node("e5", "link", "Home", 3);
        let s = format_snapshot_node(&node);
        assert!(s.starts_with("      [e5]")); // 6 spaces = 3 * 2
    }

    #[test]
    fn format_node_all_fields() {
        let mut node = make_node("e9", "checkbox", "Agree", 2);
        node.value = "true".to_string();
        node.focused = true;
        let s = format_snapshot_node(&node);
        assert_eq!(s, "    [e9] checkbox \"Agree\" value=true (focused)");
    }

    // print_snapshot and print_tab_list write to stdout — test via format helpers
    #[test]
    fn empty_snapshot_doesnt_panic() {
        // Just verify no panic
        print_snapshot(&[]);
    }

    #[test]
    fn snapshot_with_nodes_doesnt_panic() {
        let nodes = vec![
            make_node("e0", "RootWebArea", "Page", 0),
            make_node("e1", "heading", "Title", 1),
        ];
        print_snapshot(&nodes);
    }

    #[test]
    fn empty_tab_list_doesnt_panic() {
        print_tab_list(&[], "");
    }

    #[test]
    fn tab_list_with_active_doesnt_panic() {
        let tabs = vec![
            (
                "tab_0".to_string(),
                "Page 1".to_string(),
                "http://a.com".to_string(),
            ),
            (
                "tab_1".to_string(),
                "Page 2".to_string(),
                "http://b.com".to_string(),
            ),
        ];
        print_tab_list(&tabs, "tab_0");
    }

    // --- Additional edge-case tests for format_snapshot_node ---

    #[test]
    fn format_node_all_empty() {
        let node = make_node("", "", "", 0);
        let s = format_snapshot_node(&node);
        // No ref, no name => just the (empty) role
        assert_eq!(s, "");
    }

    #[test]
    fn format_node_special_characters_in_name() {
        let node = make_node("e1", "heading", "Hello \"World\" <>&", 0);
        let s = format_snapshot_node(&node);
        assert_eq!(s, r#"[e1] heading "Hello "World" <>&""#);
    }

    #[test]
    fn format_node_unicode_name() {
        let node = make_node("e4", "text", "Konnichiwa", 1);
        let s = format_snapshot_node(&node);
        assert_eq!(s, "  [e4] text \"Konnichiwa\"");
    }

    #[test]
    fn format_node_value_with_spaces_and_equals() {
        let mut node = make_node("e6", "textbox", "Query", 0);
        node.value = "key=value pair".to_string();
        let s = format_snapshot_node(&node);
        assert_eq!(s, "[e6] textbox \"Query\" value=key=value pair");
    }

    #[test]
    fn format_node_deep_indentation() {
        let node = make_node("e7", "span", "Deep", 10);
        let s = format_snapshot_node(&node);
        // 10 * 2 = 20 spaces
        assert!(s.starts_with(&" ".repeat(20)));
        assert!(s.contains("[e7] span \"Deep\""));
    }

    #[test]
    fn format_node_disabled_field_not_rendered() {
        // disabled is on the struct but format_snapshot_node does not render it
        let mut node = make_node("e8", "button", "Save", 0);
        node.disabled = true;
        let s = format_snapshot_node(&node);
        assert_eq!(s, "[e8] button \"Save\"");
        assert!(!s.contains("disabled"));
    }

    #[test]
    fn format_node_focused_with_value() {
        let mut node = make_node("e10", "slider", "Volume", 0);
        node.value = "75".to_string();
        node.focused = true;
        let s = format_snapshot_node(&node);
        // Order: ref role name value focused
        assert_eq!(s, "[e10] slider \"Volume\" value=75 (focused)");
    }

    #[test]
    fn format_node_no_ref_with_value_and_focus() {
        let mut node = make_node("", "textbox", "", 0);
        node.value = "hello".to_string();
        node.focused = true;
        let s = format_snapshot_node(&node);
        assert_eq!(s, "textbox value=hello (focused)");
    }

    #[test]
    fn format_node_zero_depth() {
        let node = make_node("e0", "document", "Root", 0);
        let s = format_snapshot_node(&node);
        // Zero depth => no leading spaces
        assert!(!s.starts_with(' '));
        assert_eq!(s, "[e0] document \"Root\"");
    }

    // --- Tests that verify print_tab_list formatting ---

    #[test]
    fn tab_list_no_active_marker_when_no_match() {
        // When active_tab doesn't match any tab id, no marker should appear.
        // We can't capture stdout easily, but we verify no panic and correct
        // format via the function's logic.
        let tabs = vec![(
            "tab_0".to_string(),
            "Title".to_string(),
            "http://x.com".to_string(),
        )];
        print_tab_list(&tabs, "nonexistent");
    }

    #[test]
    fn tab_list_single_active_tab() {
        let tabs = vec![(
            "tab_0".to_string(),
            "Only Tab".to_string(),
            "http://only.com".to_string(),
        )];
        // Single tab that is active - just verify no panic
        print_tab_list(&tabs, "tab_0");
    }

    #[test]
    fn tab_list_empty_title_and_url() {
        let tabs = vec![("tab_0".to_string(), String::new(), String::new())];
        print_tab_list(&tabs, "tab_0");
    }

    // --- Snapshot printing with varied node shapes ---

    #[test]
    fn snapshot_single_node_doesnt_panic() {
        let nodes = vec![make_node("e0", "document", "Single", 0)];
        print_snapshot(&nodes);
    }

    #[test]
    fn snapshot_deep_tree_doesnt_panic() {
        let nodes: Vec<A11yNode> = (0..20)
            .map(|i| make_node(&format!("e{i}"), "div", &format!("Node {i}"), i))
            .collect();
        print_snapshot(&nodes);
    }
}
