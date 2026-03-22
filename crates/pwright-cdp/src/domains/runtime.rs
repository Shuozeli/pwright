//! Runtime domain — JavaScript evaluation and function invocation.

use serde_json::Value;

use crate::connection::Result;
use crate::generated::runtime as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Evaluate a JavaScript expression in the page context.
    ///
    /// Returns an error if the expression throws a JavaScript exception.
    pub async fn runtime_evaluate(&self, expression: &str) -> Result<Value> {
        let params = cdp_gen::EvaluateParams {
            expression: expression.to_string(),
            return_by_value: Some(true),
            ..Default::default()
        };
        let result = self
            .send("Runtime.evaluate", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }

    /// Evaluate a JavaScript expression, returning a remote object reference.
    ///
    /// Unlike `runtime_evaluate` (which returns by value), this returns the
    /// raw CDP result with `objectId` for DOM elements. Required for JS-based
    /// selector resolution where we need the objectId to call DOM.requestNode.
    pub async fn runtime_evaluate_as_object(&self, expression: &str) -> Result<Value> {
        let params = cdp_gen::EvaluateParams {
            expression: expression.to_string(),
            return_by_value: Some(false),
            ..Default::default()
        };
        let result = self
            .send("Runtime.evaluate", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
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
        // Arguments arrive pre-formatted as CallArgument JSON objects ({"value": X}).
        // Deserialize them into the generated type rather than double-wrapping.
        let typed_args: Vec<cdp_gen::CallArgument> = arguments
            .into_iter()
            .map(|v| serde_json::from_value(v).map_err(crate::connection::CdpError::Json))
            .collect::<crate::connection::Result<Vec<_>>>()?;
        let params = cdp_gen::CallFunctionOnParams {
            function_declaration: function_declaration.to_string(),
            object_id: Some(object_id.to_string()),
            arguments: Some(typed_args),
            return_by_value: Some(true),
            ..Default::default()
        };
        let result = self
            .send("Runtime.callFunctionOn", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }

    pub async fn runtime_enable(&self) -> Result<()> {
        self.send("Runtime.enable", serde_json::json!({})).await?;
        Ok(())
    }

    /// Evaluate a JavaScript expression, awaiting any returned Promise.
    ///
    /// Like `runtime_evaluate` but passes `awaitPromise: true` to CDP,
    /// so expressions like `fetch(...).then(r => r.text())` resolve to
    /// the final string value instead of an opaque Promise object.
    pub async fn runtime_evaluate_async(&self, expression: &str) -> Result<Value> {
        let params = cdp_gen::EvaluateParams {
            expression: expression.to_string(),
            return_by_value: Some(true),
            await_promise: Some(true),
            ..Default::default()
        };
        let result = self
            .send("Runtime.evaluate", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }
}

/// Check for a JavaScript exception in a CDP result and return an error if present.
fn check_js_exception(result: &Value) -> Result<()> {
    if let Some(details) = result.get("exceptionDetails") {
        return Err(crate::connection::CdpError::JsException(
            format_js_exception(details),
        ));
    }
    Ok(())
}

/// Format a CDP exceptionDetails object into a human-readable error message.
fn format_js_exception(details: &serde_json::Value) -> String {
    let text = details
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("JavaScript exception");
    let line = details.get("lineNumber").and_then(|l| l.as_i64());
    let col = details.get("columnNumber").and_then(|c| c.as_i64());

    let mut msg = text.to_string();
    if let Some(l) = line {
        msg.push_str(&format!(" at line {l}"));
    }
    if let Some(c) = col {
        msg.push_str(&format!(":{c}"));
    }
    msg
}
