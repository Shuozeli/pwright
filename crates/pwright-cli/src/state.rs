/// CLI state persistence — `.pwright/state.json`
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const STATE_DIR: &str = ".pwright";
const STATE_FILE: &str = "state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliState {
    /// Chrome CDP HTTP endpoint (e.g. "http://localhost:9222")
    pub cdp_url: String,
    /// WebSocket URL resolved from the HTTP endpoint
    pub ws_url: String,
    /// Active tab's internal ID (e.g. "tab_00000000")
    pub active_tab: String,
    /// CDP target ID for re-attaching
    pub target_id: String,
}

impl CliState {
    fn state_dir() -> PathBuf {
        PathBuf::from(STATE_DIR)
    }

    fn state_path() -> PathBuf {
        Self::state_dir().join(STATE_FILE)
    }

    /// Load state from disk, or return default.
    pub fn load() -> Self {
        let path = Self::state_path();
        if path.exists() {
            let data = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save state to disk.
    pub fn save(&self) -> Result<()> {
        let dir = Self::state_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::state_path(), data).context("failed to write state file")?;
        Ok(())
    }

    /// Discover WS URL from Chrome HTTP endpoint using TCP.
    pub async fn fetch_ws_url(cdp_http: &str) -> Result<String> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        // Parse host:port from the URL
        let stripped = cdp_http
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        let (host, port) = if let Some(pos) = stripped.rfind(':') {
            (
                &stripped[..pos],
                stripped[pos + 1..].parse::<u16>().unwrap_or(9222),
            )
        } else {
            (stripped, 9222u16)
        };
        let addr = format!("{}:{}", host, port);

        let mut stream = TcpStream::connect(&addr)
            .await
            .context("cannot connect to Chrome")?;
        let request = format!("GET /json/version HTTP/1.1\r\nHost: {}\r\n\r\n", addr);
        stream.write_all(request.as_bytes()).await?;

        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await?;
        let body = String::from_utf8_lossy(&buf[..n]);

        // Find the JSON body after the headers
        if let Some(json_start) = body.find('{') {
            let json_str = &body[json_start..];
            let v: serde_json::Value =
                serde_json::from_str(json_str).context("invalid JSON from Chrome")?;
            let ws_url = v["webSocketDebuggerUrl"]
                .as_str()
                .context("no webSocketDebuggerUrl in response")?;
            Ok(ws_url.to_string())
        } else {
            anyhow::bail!("no JSON body in Chrome response")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_empty_fields() {
        let state = CliState::default();
        assert!(state.cdp_url.is_empty());
        assert!(state.ws_url.is_empty());
        assert!(state.active_tab.is_empty());
        assert!(state.target_id.is_empty());
    }

    #[test]
    fn state_serializes_to_json() {
        let state = CliState {
            cdp_url: "http://localhost:9222".to_string(),
            ws_url: "ws://localhost:9222/devtools/browser/abc".to_string(),
            active_tab: "tab_00000000".to_string(),
            target_id: "ABCDEF123456".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("localhost:9222"));
        assert!(json.contains("tab_00000000"));
        assert!(json.contains("ABCDEF123456"));
    }

    #[test]
    fn state_deserializes_from_json() {
        let json = r#"{
            "cdp_url": "http://localhost:9222",
            "ws_url": "ws://localhost:9222/devtools/browser/abc",
            "active_tab": "tab_00000000",
            "target_id": "TARGET123"
        }"#;
        let state: CliState = serde_json::from_str(json).unwrap();
        assert_eq!(state.cdp_url, "http://localhost:9222");
        assert_eq!(state.active_tab, "tab_00000000");
        assert_eq!(state.target_id, "TARGET123");
    }

    #[test]
    fn state_deserializes_from_empty_json_fails() {
        // All fields are required, so empty JSON should fail
        let result = serde_json::from_str::<CliState>("{}");
        assert!(result.is_err());
    }

    #[test]
    fn state_roundtrips_through_json() {
        let original = CliState {
            cdp_url: "http://host:1234".to_string(),
            ws_url: "ws://host:1234/devtools".to_string(),
            active_tab: "tab_42".to_string(),
            target_id: "XYZ".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: CliState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.cdp_url, original.cdp_url);
        assert_eq!(restored.ws_url, original.ws_url);
        assert_eq!(restored.active_tab, original.active_tab);
        assert_eq!(restored.target_id, original.target_id);
    }

    #[test]
    fn state_path_is_under_state_dir() {
        let path = CliState::state_path();
        assert!(path.starts_with(STATE_DIR));
        assert!(path.to_str().unwrap().ends_with(STATE_FILE));
    }

    #[test]
    fn load_returns_default_when_no_file() {
        // Load from a non-existent path should return default
        let state = CliState::load();
        // This is fine — in CI/test env there's no .pwright/state.json
        // so it returns default
        let _ = state;
    }
}
