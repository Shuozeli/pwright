//! Browser-root Storage-domain cookie commands.
//!
//! Cookie ownership is BrowserContext-scoped in Chromium. These commands must
//! therefore run on the browser session, not on an arbitrary Page session.

use serde::{Deserialize, Serialize};

use crate::connection::Result;
use crate::generated::network::{CookieParam, CookiePartitionKey, CookieSameSite};
use crate::session::CdpSession;

const GET_COOKIES: &str = "Storage.getCookies";
const SET_COOKIES: &str = "Storage.setCookies";
const CLEAR_COOKIES: &str = "Storage.clearCookies";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ContextParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    browser_context_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SetCookiesParams<'a> {
    cookies: &'a [CookieParam],
    #[serde(skip_serializing_if = "Option::is_none")]
    browser_context_id: Option<&'a str>,
}

#[derive(Deserialize)]
struct GetCookiesReturns {
    cookies: Vec<StorageCookie>,
}

/// Stable subset returned by `Storage.getCookies` and consumed by public
/// BrowserContext cookie APIs. Newer Chrome versions may omit unrelated
/// experimental fields from the generated Network.Cookie schema.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
    #[serde(default)]
    pub same_site: Option<CookieSameSite>,
    #[serde(default)]
    pub partition_key: Option<CookiePartitionKey>,
}

fn native_context_id(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

impl CdpSession {
    /// Read all cookies owned by one Chromium BrowserContext.
    pub async fn storage_get_cookies(
        &self,
        browser_context_id: &str,
    ) -> Result<Vec<StorageCookie>> {
        let response = self
            .send(
                GET_COOKIES,
                serde_json::to_value(ContextParams {
                    browser_context_id: native_context_id(browser_context_id),
                })?,
            )
            .await?;
        serde_json::from_value::<GetCookiesReturns>(response)
            .map(|response| response.cookies)
            .map_err(crate::connection::CdpError::Json)
    }

    /// Set cookies in exactly one Chromium BrowserContext.
    pub async fn storage_set_cookies(
        &self,
        browser_context_id: &str,
        cookies: &[CookieParam],
    ) -> Result<()> {
        self.send(
            SET_COOKIES,
            serde_json::to_value(SetCookiesParams {
                cookies,
                browser_context_id: native_context_id(browser_context_id),
            })?,
        )
        .await
        .map(|_| ())
    }

    /// Clear every cookie owned by one Chromium BrowserContext.
    pub async fn storage_clear_cookies(&self, browser_context_id: &str) -> Result<()> {
        self.send(
            CLEAR_COOKIES,
            serde_json::to_value(ContextParams {
                browser_context_id: native_context_id(browser_context_id),
            })?,
        )
        .await
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_context_omits_the_empty_native_id() {
        // Arrange
        let params = ContextParams {
            browser_context_id: native_context_id(""),
        };

        // Act
        let value = serde_json::to_value(params).unwrap();

        // Assert
        assert_eq!(value, serde_json::json!({}));
    }

    #[test]
    fn owned_context_keeps_the_exact_native_id() {
        // Arrange
        let params = ContextParams {
            browser_context_id: native_context_id("native-context-7"),
        };

        // Act
        let value = serde_json::to_value(params).unwrap();

        // Assert
        assert_eq!(
            value,
            serde_json::json!({ "browserContextId": "native-context-7" })
        );
    }

    #[test]
    fn storage_cookie_accepts_chrome_response_without_experimental_fields() {
        // Arrange
        let value = serde_json::json!({
            "name": "session",
            "value": "value",
            "domain": "example.test",
            "path": "/",
            "expires": -1,
            "httpOnly": false,
            "secure": false
        });

        // Act
        let cookie: StorageCookie = serde_json::from_value(value).unwrap();

        // Assert
        assert_eq!(cookie.name, "session");
        assert!(cookie.same_site.is_none());
        assert!(cookie.partition_key.is_none());
    }
}
