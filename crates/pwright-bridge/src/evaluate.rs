//! JavaScript evaluation.

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use serde_json::Value;

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
}
