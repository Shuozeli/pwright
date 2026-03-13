//! Network domain — cookies, resource blocking.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::connection::Result;
use crate::session::CdpSession;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    #[serde(default)]
    pub expires: f64,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub secure: bool,
    #[serde(default)]
    pub same_site: String,
}

impl CdpSession {
    /// Enable Network domain events.
    pub async fn network_enable(&self) -> Result<()> {
        self.send("Network.enable", json!({})).await?;
        Ok(())
    }

    /// Block URLs matching patterns.
    pub async fn network_set_blocked_urls(&self, patterns: &[String]) -> Result<()> {
        self.send("Network.setBlockedURLs", json!({ "urls": patterns }))
            .await?;
        Ok(())
    }

    /// Get all cookies for the current page.
    pub async fn network_get_cookies(&self) -> Result<Vec<Cookie>> {
        let result = self.send("Network.getCookies", json!({})).await?;
        let cookies: Vec<Cookie> =
            serde_json::from_value(result["cookies"].clone()).unwrap_or_default();
        Ok(cookies)
    }

    /// Set cookies.
    pub async fn network_set_cookies(&self, cookies: Vec<Value>) -> Result<()> {
        self.send("Network.setCookies", json!({ "cookies": cookies }))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cookie_deserializes_camel_case() {
        let json = serde_json::json!({
            "name": "session",
            "value": "abc123",
            "domain": ".example.com",
            "path": "/",
            "expires": 1700000000.0,
            "httpOnly": true,
            "secure": true,
            "sameSite": "Lax"
        });
        let cookie: Cookie = serde_json::from_value(json).unwrap();
        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.value, "abc123");
        assert_eq!(cookie.domain, ".example.com");
        assert!(cookie.http_only);
        assert!(cookie.secure);
        assert_eq!(cookie.same_site, "Lax");
        assert_eq!(cookie.expires, 1700000000.0);
    }

    #[test]
    fn cookie_defaults_optional_fields() {
        let json = serde_json::json!({
            "name": "test",
            "value": "v",
            "domain": "d",
            "path": "/"
        });
        let cookie: Cookie = serde_json::from_value(json).unwrap();
        assert!(!cookie.http_only);
        assert!(!cookie.secure);
        assert!(cookie.same_site.is_empty());
        assert_eq!(cookie.expires, 0.0);
    }

    #[test]
    fn cookie_roundtrips() {
        let cookie = Cookie {
            name: "test".to_string(),
            value: "val".to_string(),
            domain: ".test.com".to_string(),
            path: "/api".to_string(),
            expires: 999.0,
            http_only: false,
            secure: true,
            same_site: "Strict".to_string(),
        };
        let json = serde_json::to_value(&cookie).unwrap();
        let restored: Cookie = serde_json::from_value(json).unwrap();
        assert_eq!(restored.name, "test");
        assert_eq!(restored.same_site, "Strict");
        assert!(restored.secure);
    }

    #[test]
    fn cookie_serializes_to_camel_case() {
        let cookie = Cookie {
            name: "x".to_string(),
            value: "y".to_string(),
            domain: "d".to_string(),
            path: "/".to_string(),
            expires: 0.0,
            http_only: true,
            secure: false,
            same_site: "None".to_string(),
        };
        let json = serde_json::to_value(&cookie).unwrap();
        // Verify camelCase
        assert!(json.get("httpOnly").is_some());
        assert!(json.get("sameSite").is_some());
        assert!(json.get("http_only").is_none());
    }
}
