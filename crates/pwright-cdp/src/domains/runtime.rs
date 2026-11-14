//! Runtime domain — JavaScript evaluation and function invocation.

use serde_json::Value;

use crate::connection::Result;
use crate::generated::runtime as cdp_gen;
use crate::session::CdpSession;

impl CdpSession {
    /// One fixed binding name per target session. Repeated callers share the
    /// registration; target detach retires it without per-action cleanup.
    pub async fn runtime_ensure_binding(&self, name: &'static str) -> Result<()> {
        #[cfg(not(feature = "runtime-binding"))]
        {
            let _ = name;
            Err(crate::connection::CdpError::Other(
                "runtime-binding is not selected for this CDP client".into(),
            ))
        }
        #[cfg(feature = "runtime-binding")]
        {
            if name.is_empty() || name.len() > 64 {
                return Err(crate::connection::CdpError::Other(
                    "Runtime binding name must contain 1..=64 bytes".into(),
                ));
            }
            let installed = self
                .runtime_binding
                .get_or_try_init(|| async {
                    self.send("Runtime.addBinding", serde_json::json!({ "name": name }))
                        .await?;
                    Ok::<_, crate::connection::CdpError>(name)
                })
                .await?;
            if *installed != name {
                return Err(crate::connection::CdpError::Other(
                    "CDP target session already owns a different Runtime binding".into(),
                ));
            }
            Ok(())
        }
    }

    pub async fn runtime_evaluate_with_user_gesture(
        &self,
        expression: &str,
        unique_context_id: Option<&str>,
    ) -> Result<Value> {
        let params = user_script_evaluate_params(expression, unique_context_id);
        let result = self
            .send("Runtime.evaluate", serde_json::to_value(params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }
    /// Evaluate a JavaScript expression in the page context.
    ///
    /// Passes `awaitPromise: true` so Promise-returning expressions resolve
    /// automatically. This is a no-op for synchronous expressions.
    /// Returns an error if the expression throws a JavaScript exception.
    pub async fn runtime_evaluate(&self, expression: &str) -> Result<Value> {
        self.runtime_evaluate_with_unique_context(expression, None)
            .await
    }

    /// Evaluate a JavaScript expression in one exact execution context.
    ///
    /// CDP's globally unique context identifier is used instead of the numeric
    /// execution-context ID so a navigation cannot reuse an ID and redirect a
    /// delayed command into a replacement document.
    pub async fn runtime_evaluate_in_unique_context(
        &self,
        expression: &str,
        unique_context_id: &str,
    ) -> Result<Value> {
        self.runtime_evaluate_with_unique_context(expression, Some(unique_context_id))
            .await
    }

    async fn runtime_evaluate_with_unique_context(
        &self,
        expression: &str,
        unique_context_id: Option<&str>,
    ) -> Result<Value> {
        let params = evaluate_params(expression, unique_context_id);
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
        self.runtime_evaluate_as_object_with_unique_context(expression, None)
            .await
    }

    /// Preserve the complete retained-result response, including exception
    /// RemoteObjects, until settlement. The composing caller owns cleanup and
    /// retains this future when its public waiter is cancelled or times out.
    pub async fn runtime_evaluate_as_object_to_settlement(
        &self,
        expression: &str,
    ) -> Result<Value> {
        let params = cdp_gen::EvaluateParams {
            expression: expression.to_owned(),
            return_by_value: Some(false),
            await_promise: Some(true),
            ..Default::default()
        };
        self.send_to_settlement("Runtime.evaluate", serde_json::to_value(params)?)
            .await
    }

    /// Evaluate into a retained RemoteObject in one exact execution context.
    pub async fn runtime_evaluate_as_object_in_unique_context(
        &self,
        expression: &str,
        unique_context_id: &str,
    ) -> Result<Value> {
        self.runtime_evaluate_as_object_with_unique_context(expression, Some(unique_context_id))
            .await
    }

    async fn runtime_evaluate_as_object_with_unique_context(
        &self,
        expression: &str,
        unique_context_id: Option<&str>,
    ) -> Result<Value> {
        let params = cdp_gen::EvaluateParams {
            expression: expression.to_string(),
            return_by_value: Some(false),
            await_promise: Some(true),
            unique_context_id: unique_context_id.map(str::to_owned),
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
        let params =
            call_function_on_params(Some(object_id), None, function_declaration, arguments, true)?;
        let result = self
            .send("Runtime.callFunctionOn", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }

    /// Call a function in one exact execution context and retain its result.
    ///
    /// The supplied `CallArgument` JSON values may contain `objectId` fields,
    /// but CDP requires each referenced object to belong to this same world.
    pub async fn runtime_call_function_as_object_in_unique_context(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value> {
        let params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            arguments,
            false,
        )?;
        let result = self
            .send("Runtime.callFunctionOn", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }

    /// Raw retained-result acquisition in the exact world. No exception is
    /// reduced to text before the caller can account for its native wrappers.
    /// The caller must retain this future through settlement and cleanup.
    pub async fn runtime_call_function_as_object_in_unique_context_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value> {
        let params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            arguments,
            false,
        )?;
        self.send_to_settlement("Runtime.callFunctionOn", serde_json::to_value(params)?)
            .await
    }

    /// Call a function in one exact execution context and return its result by
    /// value. The function itself is responsible for producing a bounded,
    /// serializable envelope.
    pub async fn runtime_call_function_by_value_in_unique_context(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
    ) -> Result<Value> {
        let params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            arguments,
            true,
        )?;
        let result = self
            .send("Runtime.callFunctionOn", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }

    /// Retain the result of a synchronous call in a pre-owned group, including
    /// all exception wrappers. This explicit path preserves the raw reply and
    /// does not await returned Promises or use the ordinary response deadline.
    /// The caller must retain its group and future through native settlement.
    pub async fn runtime_call_function_as_object_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> Result<Value> {
        let mut params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            arguments,
            false,
        )?;
        params.object_group = Some(object_group.to_owned());
        params.await_promise = Some(false);
        params.user_gesture = Some(user_gesture);
        self.send_to_settlement("Runtime.callFunctionOn", serde_json::to_value(params)?)
            .await
    }

    /// Await and retain the resolved result in a pre-owned object group.
    /// The caller releases the group after bounded by-value projections.
    pub async fn runtime_call_function_as_object_awaited_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> Result<Value> {
        let mut params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            arguments,
            false,
        )?;
        params.object_group = Some(object_group.to_owned());
        params.await_promise = Some(true);
        params.user_gesture = Some(user_gesture);
        self.send_to_settlement("Runtime.callFunctionOn", serde_json::to_value(params)?)
            .await
    }

    /// Preserve the raw result/exception envelope while awaiting the returned
    /// Promise. All derived wrappers belong to the caller's pre-owned group;
    /// no ordinary response deadline can abandon their settlement.
    pub async fn runtime_call_function_raw_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> Result<Value> {
        let mut params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            arguments,
            true,
        )?;
        params.object_group = Some(object_group.to_owned());
        params.user_gesture = Some(user_gesture);
        self.send_to_settlement("Runtime.callFunctionOn", serde_json::to_value(params)?)
            .await
    }

    /// Return a value while assigning remote exception wrappers to the caller's
    /// object group. The caller retains and releases the group on every outcome.
    /// This explicitly selected path has no ordinary command response timer.
    pub async fn runtime_call_function_by_value_in_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
    ) -> Result<Value> {
        let mut params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            arguments,
            true,
        )?;
        params.object_group = Some(object_group.to_owned());
        let result = self
            .send_to_settlement("Runtime.callFunctionOn", serde_json::to_value(params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }

    /// Requires both compile-time and production-profile large-command selection.
    pub async fn runtime_call_function_by_value_in_large_group_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
    ) -> Result<Value> {
        #[cfg(feature = "large-commands")]
        {
            let mut params = call_function_on_params(
                None,
                Some(unique_context_id),
                function_declaration,
                arguments,
                true,
            )?;
            params.object_group = Some(object_group.to_owned());
            let result = self
                .send_large_to_settlement("Runtime.callFunctionOn", serde_json::to_value(params)?)
                .await?;
            check_js_exception(&result)?;
            Ok(result)
        }
        #[cfg(not(feature = "large-commands"))]
        {
            let _ = (
                unique_context_id,
                function_declaration,
                arguments,
                object_group,
            );
            Err(crate::connection::CdpError::InvalidConfiguration(
                "large commands are not compiled in",
            ))
        }
    }

    /// Requires the separately admitted large-command pool and preserves an
    /// explicit user-gesture policy for internal utility evaluation.
    pub async fn runtime_call_function_by_value_in_large_group_with_user_gesture_to_settlement(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        arguments: Vec<Value>,
        object_group: &str,
        user_gesture: bool,
    ) -> Result<Value> {
        #[cfg(feature = "large-commands")]
        {
            let mut params = call_function_on_params(
                None,
                Some(unique_context_id),
                function_declaration,
                arguments,
                true,
            )?;
            params.object_group = Some(object_group.to_owned());
            params.user_gesture = Some(user_gesture);
            let result = self
                .send_large_to_settlement("Runtime.callFunctionOn", serde_json::to_value(params)?)
                .await?;
            check_js_exception(&result)?;
            Ok(result)
        }
        #[cfg(not(feature = "large-commands"))]
        {
            let _ = (
                unique_context_id,
                function_declaration,
                arguments,
                object_group,
                user_gesture,
            );
            Err(crate::connection::CdpError::InvalidConfiguration(
                "large commands are not compiled in",
            ))
        }
    }

    /// Read own property descriptors for one retained object.
    pub async fn runtime_get_properties(&self, object_id: &str) -> Result<Value> {
        let params = cdp_gen::GetPropertiesParams {
            object_id: object_id.to_owned(),
            own_properties: Some(true),
            accessor_properties_only: None,
            generate_preview: Some(true),
            non_indexed_properties_only: None,
        };
        let result = self
            .send("Runtime.getProperties", serde_json::to_value(&params)?)
            .await?;
        check_js_exception(&result)?;
        Ok(result)
    }

    /// Preserve raw descriptor/exception owners and the response registration
    /// until settlement. Cancellation cleanup belongs to the selected caller.
    pub async fn runtime_get_properties_to_settlement(&self, object_id: &str) -> Result<Value> {
        let params = cdp_gen::GetPropertiesParams {
            object_id: object_id.to_owned(),
            own_properties: Some(true),
            accessor_properties_only: None,
            generate_preview: Some(true),
            non_indexed_properties_only: None,
        };
        self.send_to_settlement("Runtime.getProperties", serde_json::to_value(&params)?)
            .await
    }

    /// Run one synchronous retained query in a caller-owned object group.
    pub async fn runtime_query_object_in_group(
        &self,
        unique_context_id: &str,
        function_declaration: &str,
        object_group: &str,
    ) -> Result<Value> {
        let mut params = call_function_on_params(
            None,
            Some(unique_context_id),
            function_declaration,
            Vec::new(),
            false,
        )?;
        params.object_group = Some(object_group.to_owned());
        params.await_promise = Some(false);
        // Preserve exception RemoteObjects for the group's cleanup owner.
        self.send("Runtime.callFunctionOn", serde_json::to_value(params)?)
            .await
    }

    pub async fn runtime_release_object_group(&self, object_group: &str) -> Result<()> {
        let params = cdp_gen::ReleaseObjectGroupParams {
            object_group: object_group.to_owned(),
        };
        self.send("Runtime.releaseObjectGroup", serde_json::to_value(params)?)
            .await?;
        Ok(())
    }

    /// Keep the group owner until acknowledgement or explicit terminal failure.
    /// This opt-in path does not use the ordinary command response timeout.
    pub async fn runtime_release_object_group_to_settlement(
        &self,
        object_group: &str,
    ) -> Result<()> {
        let params = cdp_gen::ReleaseObjectGroupParams {
            object_group: object_group.to_owned(),
        };
        self.send_to_settlement("Runtime.releaseObjectGroup", serde_json::to_value(params)?)
            .await?;
        Ok(())
    }

    /// Release one remote JavaScript object retained by this CDP session.
    pub async fn runtime_release_object(&self, object_id: &str) -> Result<()> {
        let params = cdp_gen::ReleaseObjectParams {
            object_id: object_id.to_string(),
        };
        self.send("Runtime.releaseObject", serde_json::to_value(params)?)
            .await?;
        Ok(())
    }

    /// Await the native release acknowledgement without an ordinary response
    /// timeout. Transport failure is an error, not proof of native retirement.
    pub async fn runtime_release_object_to_settlement(&self, object_id: &str) -> Result<()> {
        let params = cdp_gen::ReleaseObjectParams {
            object_id: object_id.to_owned(),
        };
        self.send_to_settlement("Runtime.releaseObject", serde_json::to_value(params)?)
            .await?;
        Ok(())
    }

    pub async fn runtime_enable(&self) -> Result<()> {
        self.send("Runtime.enable", serde_json::json!({})).await?;
        Ok(())
    }

    /// Resumes a target created with `waitForDebuggerOnStart`.
    pub async fn runtime_run_if_waiting_for_debugger(&self) -> Result<()> {
        self.send("Runtime.runIfWaitingForDebugger", serde_json::json!({}))
            .await?;
        Ok(())
    }
}

fn evaluate_params(expression: &str, unique_context_id: Option<&str>) -> cdp_gen::EvaluateParams {
    cdp_gen::EvaluateParams {
        expression: expression.to_string(),
        return_by_value: Some(true),
        await_promise: Some(true),
        unique_context_id: unique_context_id.map(str::to_owned),
        ..Default::default()
    }
}

fn user_script_evaluate_params(
    expression: &str,
    unique_context_id: Option<&str>,
) -> cdp_gen::EvaluateParams {
    cdp_gen::EvaluateParams {
        user_gesture: Some(true),
        ..evaluate_params(expression, unique_context_id)
    }
}

#[cfg(test)]
mod user_gesture_tests {
    #[test]
    fn user_script_activation_is_explicit_and_keeps_unique_context_routing() {
        // Arrange
        let source = "navigator.userActivation.isActive";
        // Act
        let selected =
            serde_json::to_value(super::user_script_evaluate_params(source, Some("world-1")))
                .unwrap();
        let ordinary =
            serde_json::to_value(super::evaluate_params(source, Some("world-1"))).unwrap();
        // Assert
        assert_eq!(selected["userGesture"], true);
        assert_eq!(selected["uniqueContextId"], "world-1");
        assert_eq!(selected["awaitPromise"], true);
        assert_eq!(selected["returnByValue"], true);
        assert!(ordinary.get("userGesture").is_none());
    }
}

fn call_function_on_params(
    object_id: Option<&str>,
    unique_context_id: Option<&str>,
    function_declaration: &str,
    arguments: Vec<Value>,
    return_by_value: bool,
) -> Result<cdp_gen::CallFunctionOnParams> {
    // Arguments arrive pre-formatted as CallArgument JSON objects ({"value": X}).
    // Deserialize them into the generated type rather than double-wrapping.
    let typed_args = arguments
        .into_iter()
        .map(|value| serde_json::from_value(value).map_err(crate::connection::CdpError::Json))
        .collect::<crate::connection::Result<Vec<_>>>()?;
    Ok(cdp_gen::CallFunctionOnParams {
        function_declaration: function_declaration.to_owned(),
        object_id: object_id.map(str::to_owned),
        arguments: Some(typed_args),
        return_by_value: Some(return_by_value),
        await_promise: Some(true),
        unique_context_id: unique_context_id.map(str::to_owned),
        ..Default::default()
    })
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
mod tests {
    use super::{call_function_on_params, cdp_gen, evaluate_params};

    #[test]
    fn context_targeted_evaluation_uses_only_the_unique_context_id() {
        // Arrange
        let expression = "document.title";

        // Act
        let parameters = evaluate_params(expression, Some("context-unique-1"));
        let value = serde_json::to_value(parameters).expect("parameters should serialize");

        // Assert
        assert_eq!(value["expression"], expression);
        assert_eq!(value["uniqueContextId"], "context-unique-1");
        assert!(value.get("contextId").is_none());
        assert_eq!(value["awaitPromise"], true);
        assert_eq!(value["returnByValue"], true);
    }

    #[test]
    fn context_targeted_object_evaluation_retains_the_remote_object() {
        // Arrange
        let expression = "document.body";

        // Act
        let parameters = cdp_gen::EvaluateParams {
            expression: expression.to_owned(),
            return_by_value: Some(false),
            await_promise: Some(true),
            unique_context_id: Some("context-unique-1".to_owned()),
            ..Default::default()
        };
        let value = serde_json::to_value(parameters).unwrap();

        // Assert
        assert_eq!(value["expression"], expression);
        assert_eq!(value["uniqueContextId"], "context-unique-1");
        assert_eq!(value["returnByValue"], false);
        assert_eq!(value["awaitPromise"], true);
        assert!(value.get("contextId").is_none());
    }

    #[test]
    fn typed_function_calls_await_returned_promises() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let arguments = vec![serde_json::json!({"value": ["one", "two"]})];

        // Act
        let parameters = call_function_on_params(
            Some("object-1"),
            None,
            "async function(args) { return args; }",
            arguments,
            true,
        )?;
        let value = serde_json::to_value(parameters)?;

        // Assert
        assert_eq!(value["awaitPromise"], true);
        assert_eq!(value["returnByValue"], true);
        assert_eq!(value["objectId"], "object-1");
        assert_eq!(value["arguments"][0]["value"][1], "two");
        Ok(())
    }

    #[test]
    fn unique_context_function_call_retains_the_result_object()
    -> Result<(), Box<dyn std::error::Error>> {
        // Arrange
        let arguments = vec![serde_json::json!({"objectId": "node-1"})];

        // Act
        let parameters = call_function_on_params(
            None,
            Some("context-unique-1"),
            "function(node) { return node; }",
            arguments,
            false,
        )?;
        let value = serde_json::to_value(parameters)?;

        // Assert
        assert_eq!(value["uniqueContextId"], "context-unique-1");
        assert_eq!(value["returnByValue"], false);
        assert_eq!(value["arguments"][0]["objectId"], "node-1");
        assert!(value.get("objectId").is_none());
        Ok(())
    }
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
