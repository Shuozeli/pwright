//! Runtime domain — JavaScript evaluation and function invocation.

use serde_json::Value;

use crate::connection::Result;
use crate::generated::runtime as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// Evaluate a JavaScript expression in the page context.
    ///
    /// Passes `awaitPromise: true` so Promise-returning expressions resolve
    /// automatically. This is a no-op for synchronous expressions.
    /// Returns an error if the expression throws a JavaScript exception.
    pub async fn runtime_evaluate(&self, expression: &str) -> Result<Value> {
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
///
/// CDP's `ExceptionDetails` carries the raw thrown value in `exception` (a
/// RemoteObject). For `throw new Error('foo')` the useful text lives in
/// `exception.description` (which also includes the stack). For `throw 'foo'`
/// or `throw 42` the thrown value is in `exception.value`. We include both
/// when available so the caller sees the actual error, not just line:col.
fn format_js_exception(details: &serde_json::Value) -> String {
    let text = details
        .get("text")
        .and_then(|t| t.as_str())
        .unwrap_or("JavaScript exception");
    let line = details.get("lineNumber").and_then(|l| l.as_i64());
    let col = details.get("columnNumber").and_then(|c| c.as_i64());

    let exception = details.get("exception");
    let exception_desc = exception
        .and_then(|e| e.get("description"))
        .and_then(|d| d.as_str());
    let exception_value = exception
        .and_then(|e| e.get("value"))
        .map(|v| v.to_string());

    let mut msg = text.to_string();
    if let Some(desc) = exception_desc {
        // Strip trailing whitespace; the description often ends in a newline.
        msg.push_str(": ");
        msg.push_str(desc.trim_end());
    } else if let Some(val) = exception_value {
        msg.push_str(": ");
        msg.push_str(&val);
    }
    if let Some(l) = line {
        msg.push_str(&format!(" at line {l}"));
    }
    if let Some(c) = col {
        msg.push_str(&format!(":{c}"));
    }
    msg
}

#[cfg(test)]
mod format_js_exception_tests {
    use super::format_js_exception;
    use serde_json::json;

    #[test]
    fn includes_exception_description() {
        let details = json!({
            "text": "Uncaught",
            "lineNumber": 2,
            "columnNumber": 25,
            "exception": {
                "type": "object",
                "subtype": "error",
                "className": "Error",
                "description": "Error: composer not found\n    at <anonymous>:2:25",
            }
        });
        let msg = format_js_exception(&details);
        assert!(msg.contains("Error: composer not found"), "got: {msg}");
        assert!(msg.contains("line 2:25"), "got: {msg}");
    }

    #[test]
    fn falls_back_to_value_for_non_error_throws() {
        let details = json!({
            "text": "Uncaught",
            "exception": {"type": "string", "value": "some string"}
        });
        let msg = format_js_exception(&details);
        assert!(msg.contains("some string"), "got: {msg}");
    }

    #[test]
    fn preserves_old_shape_without_exception() {
        let details = json!({
            "text": "Uncaught",
            "lineNumber": 2,
            "columnNumber": 25,
        });
        let msg = format_js_exception(&details);
        assert_eq!(msg, "Uncaught at line 2:25");
    }
}
