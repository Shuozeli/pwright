//! DOM domain — focus, scroll, box model, node resolution.

use serde_json::Value;

use crate::connection::Result;
use crate::generated::dom as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Focus an element by nodeId.
    pub async fn dom_focus(&self, node_id: i64) -> Result<()> {
        let params = cdp_gen::FocusParams {
            node_id: Some(node_id),
            ..Default::default()
        };
        self.send("DOM.focus", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// Scroll an element into view by nodeId.
    pub async fn dom_scroll_into_view(&self, node_id: i64) -> Result<()> {
        let params = cdp_gen::ScrollIntoViewIfNeededParams {
            node_id: Some(node_id),
            ..Default::default()
        };
        self.send("DOM.scrollIntoViewIfNeeded", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// Get the box model for an element by nodeId.
    pub async fn dom_get_box_model(&self, node_id: i64) -> Result<Value> {
        let params = cdp_gen::GetBoxModelParams {
            node_id: Some(node_id),
            ..Default::default()
        };
        self.send("DOM.getBoxModel", serde_json::to_value(&params)?)
            .await
    }

    /// Resolve a DOM node to a JavaScript remote object by nodeId.
    pub async fn dom_resolve_node(&self, node_id: i64) -> Result<Value> {
        let params = cdp_gen::ResolveNodeParams {
            node_id: Some(node_id),
            ..Default::default()
        };
        self.send("DOM.resolveNode", serde_json::to_value(&params)?)
            .await
    }

    /// Enable the DOM domain.
    pub async fn dom_enable(&self) -> Result<()> {
        self.send(
            "DOM.enable",
            serde_json::to_value(cdp_gen::EnableParams::default())?,
        )
        .await?;
        Ok(())
    }

    /// Get the document root node.
    pub async fn dom_get_document(&self) -> Result<Value> {
        self.send(
            "DOM.getDocument",
            serde_json::to_value(cdp_gen::GetDocumentParams::default())?,
        )
        .await
    }

    /// Find a single node matching a CSS selector within the given node.
    pub async fn dom_query_selector(&self, node_id: i64, selector: &str) -> Result<i64> {
        let params = cdp_gen::QuerySelectorParams {
            node_id,
            selector: selector.to_string(),
        };
        let result = self
            .send("DOM.querySelector", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::QuerySelectorReturns = serde_json::from_value(result)?;
        Ok(returns.node_id)
    }

    /// Find all nodes matching a CSS selector within the given node.
    pub async fn dom_query_selector_all(&self, node_id: i64, selector: &str) -> Result<Vec<i64>> {
        let params = cdp_gen::QuerySelectorAllParams {
            node_id,
            selector: selector.to_string(),
        };
        let result = self
            .send("DOM.querySelectorAll", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::QuerySelectorAllReturns = serde_json::from_value(result)?;
        Ok(returns.node_ids)
    }

    /// Get attributes for a node as key-value pairs.
    pub async fn dom_get_attributes(&self, node_id: i64) -> Result<Vec<String>> {
        let params = cdp_gen::GetAttributesParams { node_id };
        let result = self
            .send("DOM.getAttributes", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::GetAttributesReturns = serde_json::from_value(result)?;
        Ok(returns.attributes)
    }

    /// Get the outer HTML for a node.
    pub async fn dom_get_outer_html(&self, node_id: i64) -> Result<String> {
        let params = cdp_gen::GetOuterHTMLParams {
            node_id: Some(node_id),
            ..Default::default()
        };
        let result = self
            .send("DOM.getOuterHTML", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::GetOuterHTMLReturns = serde_json::from_value(result)?;
        Ok(returns.outer_html)
    }

    /// Describe a node by backendNodeId, returning its details.
    pub async fn dom_describe_node(&self, backend_node_id: i64) -> Result<Value> {
        let params = cdp_gen::DescribeNodeParams {
            backend_node_id: Some(backend_node_id),
            ..Default::default()
        };
        self.send("DOM.describeNode", serde_json::to_value(&params)?)
            .await
    }

    /// Set files for a file input element.
    pub async fn dom_set_file_input_files(&self, node_id: i64, files: &[String]) -> Result<()> {
        let params = cdp_gen::SetFileInputFilesParams {
            files: files.to_vec(),
            node_id: Some(node_id),
            ..Default::default()
        };
        self.send("DOM.setFileInputFiles", serde_json::to_value(&params)?)
            .await?;
        Ok(())
    }

    /// Request a DOM nodeId for a JavaScript remote object.
    pub async fn dom_request_node(&self, object_id: &str) -> Result<i64> {
        let params = cdp_gen::RequestNodeParams {
            object_id: object_id.to_string(),
        };
        let result = self
            .send("DOM.requestNode", serde_json::to_value(&params)?)
            .await?;
        let returns: cdp_gen::RequestNodeReturns = serde_json::from_value(result)?;
        Ok(returns.node_id)
    }
}
