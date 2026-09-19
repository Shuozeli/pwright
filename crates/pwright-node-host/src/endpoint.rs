use pwright_provider_cdp::CdpWebSocketEndpoint;
use serde::Deserialize;
use url::Url;

use crate::HostError;

const MAX_ENDPOINT_BYTES: usize = 8192;
const MAX_DISCOVERY_BYTES: usize = 64 * 1024;

#[derive(Deserialize)]
struct Discovery {
    #[serde(rename = "webSocketDebuggerUrl")]
    websocket: String,
}

pub fn validate_endpoint(endpoint: &str) -> Result<Url, HostError> {
    if endpoint.len() > MAX_ENDPOINT_BYTES {
        return Err(HostError::InvalidArgument(
            "CDP endpoint exceeds 8192 bytes".into(),
        ));
    }
    let url = Url::parse(endpoint).map_err(|_| {
        HostError::InvalidArgument("CDP endpoint must be an absolute HTTP(S) or WS(S) URL".into())
    })?;
    if !matches!(url.scheme(), "http" | "https" | "ws" | "wss") || url.host_str().is_none() {
        return Err(HostError::InvalidArgument(
            "CDP endpoint must use HTTP(S) or WS(S)".into(),
        ));
    }
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(HostError::InvalidArgument(
            "CDP endpoint credentials and fragments are unsupported".into(),
        ));
    }
    Ok(url)
}

pub async fn resolve(endpoint: &str) -> Result<CdpWebSocketEndpoint, HostError> {
    let mut url = validate_endpoint(endpoint)?;
    if matches!(url.scheme(), "ws" | "wss") {
        return CdpWebSocketEndpoint::parse(url.as_str()).map_err(HostError::backend);
    }
    let path = format!("{}/json/version/", url.path().trim_end_matches('/'));
    url.set_path(&path);
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(HostError::backend)?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|error| HostError::backend(error.without_url()))?
        .error_for_status()
        .map_err(|error| HostError::backend(error.without_url()))?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(HostError::backend)? {
        if chunk.len() > MAX_DISCOVERY_BYTES.saturating_sub(bytes.len()) {
            return Err(HostError::ResourceExhausted(
                "CDP discovery response exceeds 64 KiB".into(),
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    let discovery: Discovery = serde_json::from_slice(&bytes).map_err(HostError::backend)?;
    let websocket = validate_endpoint(&discovery.websocket)?;
    CdpWebSocketEndpoint::parse(websocket.as_str()).map_err(HostError::backend)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_rejected_without_echoing_the_secret() {
        // Arrange
        let endpoint = "http://user:private-value@127.0.0.1:9222";
        // Act
        let error = validate_endpoint(endpoint).expect_err("credentials must fail");
        // Assert
        assert!(matches!(error, HostError::InvalidArgument(_)));
        assert!(!error.to_string().contains("private-value"));
    }

    #[test]
    fn oversized_endpoint_is_rejected_before_url_parsing() {
        // Arrange
        let endpoint = "x".repeat(MAX_ENDPOINT_BYTES + 1);
        // Act
        let result = validate_endpoint(&endpoint);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }
}
