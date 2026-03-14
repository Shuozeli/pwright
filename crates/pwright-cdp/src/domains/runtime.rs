//! Runtime domain — JavaScript evaluation and function invocation.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::connection::Result;
use crate::session::CdpSession;

/// Typed representation of a CDP RemoteObject.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObject {
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub object_id: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// Typed result from `Runtime.evaluate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResult {
    pub result: RemoteObject,
    #[serde(default)]
    pub exception_details: Option<Value>,
}

/// Typed result from `Runtime.callFunctionOn`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallFunctionResult {
    pub result: RemoteObject,
    #[serde(default)]
    pub exception_details: Option<Value>,
}

impl CdpSession {
    /// Evaluate a JavaScript expression in the page context.
    ///
    /// Returns an error if the expression throws a JavaScript exception.
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
        if let Some(details) = result.get("exceptionDetails") {
            let text = details
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("JavaScript exception");
            return Err(crate::connection::CdpError::Other(text.to_string()));
        }
        Ok(result)
    }

    /// Evaluate a JavaScript expression, returning a remote object reference.
    ///
    /// Unlike `runtime_evaluate` (which returns by value), this returns the
    /// raw CDP result with `objectId` for DOM elements. Required for JS-based
    /// selector resolution where we need the objectId to call DOM.requestNode.
    pub async fn runtime_evaluate_as_object(&self, expression: &str) -> Result<Value> {
        let result = self
            .send(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": false,
                }),
            )
            .await?;
        if let Some(details) = result.get("exceptionDetails") {
            let text = details
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("JavaScript exception");
            return Err(crate::connection::CdpError::Other(text.to_string()));
        }
        Ok(result)
    }

    /// Call a function on a remote object.
    ///
    /// Returns an error if the function throws a JavaScript exception.
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
        if let Some(details) = result.get("exceptionDetails") {
            let text = details
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("JavaScript exception");
            return Err(crate::connection::CdpError::Other(text.to_string()));
        }
        Ok(result)
    }

    /// Enable the Runtime domain.
    pub async fn runtime_enable(&self) -> Result<()> {
        self.send("Runtime.enable", json!({})).await?;
        Ok(())
    }

    /// Evaluate a JavaScript expression, returning a typed result.
    pub async fn runtime_evaluate_typed(&self, expression: &str) -> Result<EvaluateResult> {
        let raw = self.runtime_evaluate(expression).await?;
        serde_json::from_value(raw).map_err(crate::connection::CdpError::Json)
    }

    /// Call a function on a remote object, returning a typed result.
    pub async fn runtime_call_function_typed(
        &self,
        object_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<CallFunctionResult> {
        let raw = self
            .runtime_call_function_on(object_id, function_declaration, arguments)
            .await?;
        serde_json::from_value(raw).map_err(crate::connection::CdpError::Json)
    }

    /// Evaluate a JavaScript expression, awaiting any returned Promise.
    ///
    /// Like `runtime_evaluate` but passes `awaitPromise: true` to CDP,
    /// so expressions like `fetch(...).then(r => r.text())` resolve to
    /// the final string value instead of an opaque Promise object.
    pub async fn runtime_evaluate_async(&self, expression: &str) -> Result<Value> {
        let result = self
            .send(
                "Runtime.evaluate",
                json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                }),
            )
            .await?;
        if let Some(details) = result.get("exceptionDetails") {
            let text = details
                .get("text")
                .and_then(|t| t.as_str())
                .unwrap_or("JavaScript exception");
            return Err(crate::connection::CdpError::Other(text.to_string()));
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_object_deserialize() {
        let json = serde_json::json!({
            "type": "number",
            "value": 42,
            "description": "42"
        });
        let obj: RemoteObject = serde_json::from_value(json).unwrap();
        assert_eq!(obj.object_type, "number");
        assert_eq!(obj.value, Some(serde_json::json!(42)));
        assert_eq!(obj.description, Some("42".to_string()));
        assert!(obj.object_id.is_none());
        assert!(obj.subtype.is_none());
    }

    #[test]
    fn test_evaluate_result_deserialize() {
        let json = serde_json::json!({
            "result": {
                "type": "string",
                "value": "hello"
            }
        });
        let result: EvaluateResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.result.object_type, "string");
        assert_eq!(result.result.value, Some(serde_json::json!("hello")));
        assert!(result.exception_details.is_none());
    }

    #[test]
    fn test_call_function_result_deserialize() {
        let json = serde_json::json!({
            "result": {
                "type": "boolean",
                "value": true
            }
        });
        let result: CallFunctionResult = serde_json::from_value(json).unwrap();
        assert_eq!(result.result.object_type, "boolean");
        assert_eq!(result.result.value, Some(serde_json::json!(true)));
    }

    #[test]
    fn test_evaluate_result_with_exception() {
        let json = serde_json::json!({
            "result": {
                "type": "undefined"
            },
            "exceptionDetails": {
                "text": "ReferenceError: foo is not defined"
            }
        });
        let result: EvaluateResult = serde_json::from_value(json).unwrap();
        assert!(result.exception_details.is_some());
    }

    #[test]
    fn test_remote_object_with_object_id() {
        let json = serde_json::json!({
            "type": "object",
            "subtype": "node",
            "objectId": "{\"injectedScriptId\":1,\"id\":42}"
        });
        let obj: RemoteObject = serde_json::from_value(json).unwrap();
        assert_eq!(obj.object_type, "object");
        assert_eq!(obj.subtype, Some("node".to_string()));
        assert!(obj.object_id.is_some());
    }

    #[test]
    fn test_evaluate_result_missing_optional_fields() {
        let json = serde_json::json!({
            "result": {
                "type": "undefined"
            }
        });
        let result: EvaluateResult = serde_json::from_value(json).unwrap();
        assert!(result.result.value.is_none());
        assert!(result.exception_details.is_none());
    }
}
