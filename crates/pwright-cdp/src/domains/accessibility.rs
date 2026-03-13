//! Accessibility domain — full accessibility tree retrieval.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::connection::Result;
use crate::session::CdpSession;

/// Raw accessibility node from CDP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawAXNode {
    pub node_id: String,
    #[serde(default)]
    pub ignored: bool,
    pub role: Option<AXValue>,
    pub name: Option<AXValue>,
    pub value: Option<AXValue>,
    #[serde(default)]
    pub properties: Vec<AXProperty>,
    #[serde(default)]
    pub child_ids: Vec<String>,
    /// Chrome sends this as "backendDOMNodeId" (uppercase DOM),
    /// but serde's camelCase maps our field to "backendDomNodeId".
    /// The alias accepts both spellings during deserialization.
    #[serde(default, alias = "backendDOMNodeId")]
    pub backend_dom_node_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AXValue {
    #[serde(rename = "type")]
    pub value_type: String,
    pub value: Value,
}

impl AXValue {
    pub fn as_str(&self) -> &str {
        self.value.as_str().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AXProperty {
    pub name: String,
    pub value: AXValue,
}

impl CdpSession {
    /// Enable the Accessibility domain (required before getFullAXTree).
    pub async fn accessibility_enable(&self) -> Result<()> {
        self.send("Accessibility.enable", json!({})).await?;
        Ok(())
    }

    /// Get the full accessibility tree for the current page.
    /// Passes depth=-1 to retrieve the complete tree (not just root).
    pub async fn accessibility_get_full_tree(&self) -> Result<Vec<RawAXNode>> {
        let result = self
            .send("Accessibility.getFullAXTree", json!({"depth": -1}))
            .await?;
        let nodes: Vec<RawAXNode> =
            serde_json::from_value(result["nodes"].clone()).unwrap_or_default();
        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ax_value_as_str_for_string() {
        let v = AXValue {
            value_type: "string".to_string(),
            value: Value::String("button".to_string()),
        };
        assert_eq!(v.as_str(), "button");
    }

    #[test]
    fn ax_value_as_str_for_non_string() {
        let v = AXValue {
            value_type: "boolean".to_string(),
            value: Value::Bool(true),
        };
        assert_eq!(v.as_str(), ""); // returns default
    }

    #[test]
    fn raw_ax_node_deserializes_full() {
        let json = serde_json::json!({
            "nodeId": "node-1",
            "ignored": false,
            "role": {"type": "internalRole", "value": "button"},
            "name": {"type": "computedString", "value": "Submit"},
            "properties": [
                {"name": "focused", "value": {"type": "boolean", "value": true}}
            ],
            "childIds": ["node-2", "node-3"],
            "backendDomNodeId": 42
        });
        let node: RawAXNode = serde_json::from_value(json).unwrap();
        assert_eq!(node.node_id, "node-1");
        assert!(!node.ignored);
        assert_eq!(node.role.as_ref().unwrap().as_str(), "button");
        assert_eq!(node.name.as_ref().unwrap().as_str(), "Submit");
        assert_eq!(node.child_ids.len(), 2);
        assert_eq!(node.backend_dom_node_id, 42);
        assert_eq!(node.properties.len(), 1);
        assert_eq!(node.properties[0].name, "focused");
    }

    #[test]
    fn raw_ax_node_defaults_optional_fields() {
        let json = serde_json::json!({"nodeId": "n1"});
        let node: RawAXNode = serde_json::from_value(json).unwrap();
        assert!(!node.ignored);
        assert!(node.role.is_none());
        assert!(node.name.is_none());
        assert!(node.value.is_none());
        assert!(node.properties.is_empty());
        assert!(node.child_ids.is_empty());
        assert_eq!(node.backend_dom_node_id, 0);
    }

    #[test]
    fn raw_ax_node_roundtrips() {
        let node = RawAXNode {
            node_id: "test-id".to_string(),
            ignored: true,
            role: Some(AXValue {
                value_type: "internalRole".to_string(),
                value: Value::String("heading".to_string()),
            }),
            name: None,
            value: None,
            properties: vec![],
            child_ids: vec!["c1".to_string()],
            backend_dom_node_id: 99,
        };
        let json = serde_json::to_value(&node).unwrap();
        let restored: RawAXNode = serde_json::from_value(json).unwrap();
        assert_eq!(restored.node_id, "test-id");
        assert!(restored.ignored);
        assert_eq!(restored.backend_dom_node_id, 99);
    }

    #[test]
    fn ax_property_deserializes() {
        let json = serde_json::json!({
            "name": "disabled",
            "value": {"type": "boolean", "value": false}
        });
        let prop: AXProperty = serde_json::from_value(json).unwrap();
        assert_eq!(prop.name, "disabled");
        assert_eq!(prop.value.value_type, "boolean");
    }
}
