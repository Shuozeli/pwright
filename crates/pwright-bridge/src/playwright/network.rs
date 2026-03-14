//! Network event interception for Page.
//!
//! Provides typed structs and parsing helpers for CDP Network domain events.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A network response received by the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkResponse {
    pub request_id: String,
    pub url: String,
    pub status: i64,
    pub status_text: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    pub mime_type: String,
}

/// A network request sent by the browser.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRequest {
    pub request_id: String,
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub post_data: Option<String>,
    pub resource_type: String,
}

/// Parse a `Network.responseReceived` event into a `NetworkResponse`.
pub fn parse_network_response(params: &Value) -> Option<NetworkResponse> {
    let request_id = params.get("requestId")?.as_str()?.to_string();
    let response = params.get("response")?;
    let url = response.get("url")?.as_str()?.to_string();
    let status = response.get("status")?.as_i64()?;
    let status_text = response
        .get("statusText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mime_type = response
        .get("mimeType")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let headers = response
        .get("headers")
        .and_then(|h| serde_json::from_value(h.clone()).ok())
        .unwrap_or_default();

    Some(NetworkResponse {
        request_id,
        url,
        status,
        status_text,
        headers,
        mime_type,
    })
}

/// Parse a `Network.requestWillBeSent` event into a `NetworkRequest`.
pub fn parse_network_request(params: &Value) -> Option<NetworkRequest> {
    let request_id = params.get("requestId")?.as_str()?.to_string();
    let request = params.get("request")?;
    let url = request.get("url")?.as_str()?.to_string();
    let method = request.get("method")?.as_str()?.to_string();
    let headers = request
        .get("headers")
        .and_then(|h| serde_json::from_value(h.clone()).ok())
        .unwrap_or_default();
    let post_data = request
        .get("postData")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let resource_type = params
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("Other")
        .to_string();

    Some(NetworkRequest {
        request_id,
        url,
        method,
        headers,
        post_data,
        resource_type,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_network_response() {
        let params = json!({
            "requestId": "req-1",
            "response": {
                "url": "https://example.com/api",
                "status": 200,
                "statusText": "OK",
                "headers": {"content-type": "application/json"},
                "mimeType": "application/json"
            }
        });

        let resp = parse_network_response(&params).unwrap();
        assert_eq!(resp.request_id, "req-1");
        assert_eq!(resp.url, "https://example.com/api");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.status_text, "OK");
        assert_eq!(resp.mime_type, "application/json");
        assert_eq!(
            resp.headers.get("content-type").unwrap(),
            "application/json"
        );
    }

    #[test]
    fn test_parse_network_request() {
        let params = json!({
            "requestId": "req-2",
            "request": {
                "url": "https://example.com/submit",
                "method": "POST",
                "headers": {"content-type": "application/json"},
                "postData": "{\"key\":\"value\"}"
            },
            "type": "XHR"
        });

        let req = parse_network_request(&params).unwrap();
        assert_eq!(req.request_id, "req-2");
        assert_eq!(req.url, "https://example.com/submit");
        assert_eq!(req.method, "POST");
        assert_eq!(req.post_data, Some("{\"key\":\"value\"}".to_string()));
        assert_eq!(req.resource_type, "XHR");
    }

    #[test]
    fn test_parse_network_response_missing_fields() {
        let params = json!({});
        assert!(parse_network_response(&params).is_none());
    }

    #[test]
    fn test_parse_network_request_missing_fields() {
        let params = json!({});
        assert!(parse_network_request(&params).is_none());
    }
}
