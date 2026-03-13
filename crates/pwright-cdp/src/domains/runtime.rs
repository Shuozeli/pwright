//! Runtime domain — JavaScript evaluation and function invocation.

use serde_json::{Value, json};

use crate::connection::Result;
use crate::session::CdpSession;

impl CdpSession {
    /// Evaluate a JavaScript expression in the page context.
    pub async fn runtime_evaluate(&self, expression: &str) -> Result<Value> {
        let result = self
            .send(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                }),
            )
            .await?;
        Ok(result)
    }

    /// Call a function on a remote object.
    pub async fn runtime_call_function_on(
        &self,
        object_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value> {
        let result = self
            .send(
                "Runtime.callFunctionOn",
                json!({
                    "functionDeclaration": function_declaration,
                    "objectId": object_id,
                    "arguments": arguments,
                    "returnByValue": true,
                }),
            )
            .await?;
        Ok(result)
    }

    /// Enable the Runtime domain.
    pub async fn runtime_enable(&self) -> Result<()> {
        self.send("Runtime.enable", json!({})).await?;
        Ok(())
    }
}
