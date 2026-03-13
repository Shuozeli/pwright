//! DOM domain — focus, scroll, box model, node resolution.

use serde_json::{Value, json};

use crate::connection::Result;
use crate::session::CdpSession;

impl CdpSession {
    /// Focus an element by backendNodeId.
    pub async fn dom_focus(&self, backend_node_id: i64) -> Result<()> {
        self.send("DOM.focus", json!({ "backendNodeId": backend_node_id }))
            .await?;
        Ok(())
    }

    /// Scroll an element into view by backendNodeId.
    pub async fn dom_scroll_into_view(&self, backend_node_id: i64) -> Result<()> {
        self.send(
            "DOM.scrollIntoViewIfNeeded",
            json!({ "backendNodeId": backend_node_id }),
        )
        .await?;
        Ok(())
    }

    /// Get the box model for an element. Returns the raw model object.
    pub async fn dom_get_box_model(&self, backend_node_id: i64) -> Result<Value> {
        self.send(
            "DOM.getBoxModel",
            json!({ "backendNodeId": backend_node_id }),
        )
        .await
    }

    /// Resolve a backend node to a JavaScript object.
    pub async fn dom_resolve_node(&self, backend_node_id: i64) -> Result<Value> {
        self.send(
            "DOM.resolveNode",
            json!({ "backendNodeId": backend_node_id }),
        )
        .await
    }

    /// Enable the DOM domain.
    pub async fn dom_enable(&self) -> Result<()> {
        self.send("DOM.enable", json!({})).await?;
        Ok(())
    }

    /// Get the document root node.
    pub async fn dom_get_document(&self) -> Result<Value> {
        self.send("DOM.getDocument", json!({})).await
    }

    /// Find a single node matching a CSS selector within the given node.
    pub async fn dom_query_selector(&self, node_id: i64, selector: &str) -> Result<i64> {
        let result = self
            .send(
                "DOM.querySelector",
                json!({ "nodeId": node_id, "selector": selector }),
            )
            .await?;
        Ok(result["nodeId"].as_i64().unwrap_or(0))
    }

    /// Find all nodes matching a CSS selector within the given node.
    pub async fn dom_query_selector_all(&self, node_id: i64, selector: &str) -> Result<Vec<i64>> {
        let result = self
            .send(
                "DOM.querySelectorAll",
                json!({ "nodeId": node_id, "selector": selector }),
            )
            .await?;
        let ids = result["nodeIds"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        Ok(ids)
    }

    /// Get attributes for a node as key-value pairs.
    pub async fn dom_get_attributes(&self, node_id: i64) -> Result<Vec<String>> {
        let result = self
            .send("DOM.getAttributes", json!({ "nodeId": node_id }))
            .await?;
        let attrs = result["attributes"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        Ok(attrs)
    }

    /// Get the outer HTML for a node.
    pub async fn dom_get_outer_html(&self, node_id: i64) -> Result<String> {
        let result = self
            .send("DOM.getOuterHTML", json!({ "nodeId": node_id }))
            .await?;
        Ok(result["outerHTML"].as_str().unwrap_or_default().to_string())
    }

    /// Describe a node by backendNodeId, returning its nodeId.
    pub async fn dom_describe_node(&self, backend_node_id: i64) -> Result<Value> {
        self.send(
            "DOM.describeNode",
            json!({ "backendNodeId": backend_node_id }),
        )
        .await
    }

    /// Set files for a file input element.
    pub async fn dom_set_file_input_files(&self, node_id: i64, files: &[String]) -> Result<()> {
        self.send(
            "DOM.setFileInputFiles",
            json!({ "files": files, "backendNodeId": node_id }),
        )
        .await?;
        Ok(())
    }
}
