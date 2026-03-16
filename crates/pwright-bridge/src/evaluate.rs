//! JavaScript evaluation and typed result extraction.

use pwright_cdp::CdpClient;
use pwright_cdp::connection::{CdpError, Result as CdpResult};
use serde::de::DeserializeOwned;
use serde_json::Value;

// ── Trait: FromEvalResult ──

/// Convert a CDP `RemoteObject` (the `{"type": "...", "value": ...}` shape)
/// into a typed Rust value.
///
/// Used by `Page::evaluate_into` and `Locator::evaluate_into` to provide
/// typed evaluation without per-type method proliferation.
///
/// Implemented for: `String`, `bool`, `i64`, `f64`, `Value` (passthrough),
/// and any `T: DeserializeOwned` via [`FromEvalJson`] wrapper.
pub trait FromEvalResult: Sized {
    fn from_eval_result(remote_object: &Value) -> CdpResult<Self>;
}

impl FromEvalResult for String {
    fn from_eval_result(remote_object: &Value) -> CdpResult<Self> {
        remote_object["value"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                CdpError::Other(format!(
                    "expected string from evaluate, got: {}",
                    remote_object
                ))
            })
    }
}

impl FromEvalResult for bool {
    fn from_eval_result(remote_object: &Value) -> CdpResult<Self> {
        remote_object["value"].as_bool().ok_or_else(|| {
            CdpError::Other(format!(
                "expected bool from evaluate, got: {}",
                remote_object
            ))
        })
    }
}

impl FromEvalResult for i64 {
    fn from_eval_result(remote_object: &Value) -> CdpResult<Self> {
        remote_object["value"].as_i64().ok_or_else(|| {
            CdpError::Other(format!(
                "expected i64 from evaluate, got: {}",
                remote_object
            ))
        })
    }
}

impl FromEvalResult for f64 {
    fn from_eval_result(remote_object: &Value) -> CdpResult<Self> {
        remote_object["value"].as_f64().ok_or_else(|| {
            CdpError::Other(format!(
                "expected f64 from evaluate, got: {}",
                remote_object
            ))
        })
    }
}

impl FromEvalResult for Value {
    fn from_eval_result(remote_object: &Value) -> CdpResult<Self> {
        Ok(remote_object.clone())
    }
}

/// Wrapper for deserializing JSON string results into arbitrary types.
///
/// Use when the JS expression returns a JSON string (e.g. `JSON.stringify(...)`):
///
/// ```rust,ignore
/// let items: FromEvalJson<Vec<Item>> = page.evaluate_into("JSON.stringify([...])").await?;
/// let data = items.0;
/// ```
pub struct FromEvalJson<T>(pub T);

impl<T: DeserializeOwned> FromEvalResult for FromEvalJson<T> {
    fn from_eval_result(remote_object: &Value) -> CdpResult<Self> {
        let json_str = remote_object["value"].as_str().ok_or_else(|| {
            CdpError::Other(format!(
                "expected JSON string from evaluate, got: {}",
                remote_object
            ))
        })?;
        let parsed: T = serde_json::from_str(json_str)
            .map_err(|e| CdpError::Other(format!("failed to deserialize evaluate result: {e}")))?;
        Ok(FromEvalJson(parsed))
    }
}

// ── Core evaluate functions ──

/// Evaluate a JavaScript expression and return the result.
pub async fn evaluate(session: &dyn CdpClient, expression: &str) -> CdpResult<Value> {
    let result = session.runtime_evaluate(expression).await?;
    Ok(result.get("result").cloned().unwrap_or(Value::Null))
}

/// Evaluate a JavaScript expression, awaiting any returned Promise.
///
/// Like `evaluate` but uses `awaitPromise: true`, so expressions returning
/// Promises (e.g. `fetch(...).then(r => r.text())`) resolve to the final value.
pub async fn evaluate_async(session: &dyn CdpClient, expression: &str) -> CdpResult<Value> {
    let result = session.runtime_evaluate_async(expression).await?;
    Ok(result.get("result").cloned().unwrap_or(Value::Null))
}

/// Evaluate and convert the result to a typed value.
pub async fn evaluate_into<T: FromEvalResult>(
    session: &dyn CdpClient,
    expression: &str,
) -> CdpResult<T> {
    let remote_object = evaluate(session, expression).await?;
    T::from_eval_result(&remote_object)
}

/// Evaluate (async/Promise-aware) and convert the result to a typed value.
pub async fn evaluate_async_into<T: FromEvalResult>(
    session: &dyn CdpClient,
    expression: &str,
) -> CdpResult<T> {
    let remote_object = evaluate_async(session, expression).await?;
    T::from_eval_result(&remote_object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_evaluate_returns_result_value() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "number", "value": 42}
        }));

        let result = evaluate(&mock, "1 + 1").await.unwrap();
        assert_eq!(result["type"], "number");
        assert_eq!(result["value"], 42);

        let calls = mock.calls_for("Runtime.evaluate");
        assert_eq!(calls[0].args[0], "1 + 1");
    }

    #[tokio::test]
    async fn test_evaluate_missing_result_returns_null() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({}));

        let result = evaluate(&mock, "void 0").await.unwrap();
        assert!(result.is_null());
    }

    #[tokio::test]
    async fn test_evaluate_async_returns_result_value() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "string", "value": "fetched data"}
        }));

        let result = evaluate_async(&mock, "fetch('/api').then(r => r.text())")
            .await
            .unwrap();
        assert_eq!(result["type"], "string");
        assert_eq!(result["value"], "fetched data");

        let calls = mock.calls_for("Runtime.evaluate(async)");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].args[0], "fetch('/api').then(r => r.text())");
    }

    #[tokio::test]
    async fn test_evaluate_async_missing_result_returns_null() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({}));

        let result = evaluate_async(&mock, "void 0").await.unwrap();
        assert!(result.is_null());
    }

    #[tokio::test]
    async fn test_evaluate_into_string() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "string", "value": "hello"}
        }));

        let result: String = evaluate_into(&mock, "document.title").await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn test_evaluate_into_bool() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "boolean", "value": true}
        }));

        let result: bool = evaluate_into(&mock, "true").await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_evaluate_into_i64() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "number", "value": 42}
        }));

        let result: i64 = evaluate_into(&mock, "1 + 1").await.unwrap();
        assert_eq!(result, 42);
    }

    #[tokio::test]
    async fn test_evaluate_into_f64() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "number", "value": 1.5}
        }));

        let result: f64 = evaluate_into(&mock, "1.0 + 0.5").await.unwrap();
        assert!((result - 1.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_evaluate_into_json_deserialize() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "string", "value": r##"[{"name":"a"},{"name":"b"}]"##}
        }));

        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Item {
            name: String,
        }

        let result: FromEvalJson<Vec<Item>> =
            evaluate_into(&mock, "JSON.stringify([...])").await.unwrap();
        assert_eq!(
            result.0,
            vec![Item { name: "a".into() }, Item { name: "b".into() }]
        );
    }

    #[tokio::test]
    async fn test_evaluate_into_type_mismatch_errors() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "number", "value": 42}
        }));

        let result: CdpResult<String> = evaluate_into(&mock, "1 + 1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_evaluate_into_value_passthrough() {
        let mock = MockCdpClient::new();
        mock.set_evaluate_response(serde_json::json!({
            "result": {"type": "object", "value": {"key": "val"}}
        }));

        let result: Value = evaluate_into(&mock, "({key: 'val'})").await.unwrap();
        assert_eq!(result["value"]["key"], "val");
    }
}
