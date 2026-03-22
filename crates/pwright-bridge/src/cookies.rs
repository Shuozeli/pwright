//! Cookie management.

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;
use pwright_cdp::domains::network::Cookie;

pub async fn get_cookies(session: &dyn CdpClient) -> CdpResult<Vec<Cookie>> {
    session.network_get_cookies().await
}

pub async fn set_cookies(session: &dyn CdpClient, cookies: &[Cookie]) -> CdpResult<()> {
    session.network_set_cookies(cookies).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_get_cookies_returns_mock_cookies() {
        let mock = MockCdpClient::new();
        mock.set_cookies_response(vec![Cookie {
            name: "session".to_string(),
            value: "abc123".to_string(),
            domain: ".example.com".to_string(),
            path: "/".to_string(),
            expires: 0.0,
            http_only: true,
            secure: true,
            same_site: "Lax".to_string(),
        }]);

        let cookies = get_cookies(&mock).await.unwrap();
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "session");
        assert_eq!(cookies[0].value, "abc123");
        assert!(cookies[0].http_only);
    }

    #[tokio::test]
    async fn test_set_cookies_forwards_to_network() {
        let mock = MockCdpClient::new();
        let cookies = vec![Cookie {
            name: "test".to_string(),
            value: "123".to_string(),
            domain: String::new(),
            path: "/".to_string(),
            expires: 0.0,
            http_only: false,
            secure: false,
            same_site: String::new(),
        }];

        set_cookies(&mock, &cookies).await.unwrap();

        let calls = mock.calls_for("Network.setCookies");
        assert_eq!(calls.len(), 1);
    }
}
