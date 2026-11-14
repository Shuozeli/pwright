use pwright_transport_ws::{WsError, WsHeaders, WsRequest};

trait HeaderTarget {
    fn headers(&self) -> &WsHeaders;
    fn add_header(&mut self, name: &str, value: &str) -> Result<(), WsError>;
}

impl HeaderTarget for WsRequest {
    fn headers(&self) -> &WsHeaders {
        WsRequest::headers(self)
    }

    fn add_header(&mut self, name: &str, value: &str) -> Result<(), WsError> {
        WsRequest::add_header(self, name, value)
    }
}

impl HeaderTarget for WsHeaders {
    fn headers(&self) -> &WsHeaders {
        self
    }

    fn add_header(&mut self, name: &str, value: &str) -> Result<(), WsError> {
        self.push_for_http_discovery(name, value)
    }
}

fn has_header<T: HeaderTarget>(target: &T, name: &str) -> bool {
    target
        .headers()
        .iter()
        .any(|header| header.name().eq_ignore_ascii_case(name))
}

pub fn needs_default_user_agent(request: &WsRequest) -> bool {
    !has_header(request, "user-agent")
}

/// Apply browser-facing defaults after explicit headers. The caller's header
/// wins regardless of case; every synthesized field uses the same transport
/// count and byte budget as an explicit field.
pub fn append_server_connect_defaults(
    request: &mut WsRequest,
    browser_name: &str,
    default_user_agent: &str,
) -> Result<(), WsError> {
    append_defaults(request, browser_name, default_user_agent)
}

/// Preflight defaults under the same count and byte limits, but defer only
/// handshake-reserved names until HTTP endpoint discovery has completed.
pub fn append_server_connect_defaults_for_http_discovery(
    headers: &mut WsHeaders,
    browser_name: &str,
    default_user_agent: &str,
) -> Result<(), WsError> {
    append_defaults(headers, browser_name, default_user_agent)
}

fn append_defaults<T: HeaderTarget>(
    target: &mut T,
    browser_name: &str,
    default_user_agent: &str,
) -> Result<(), WsError> {
    if !matches!(browser_name, "chromium" | "firefox" | "webkit") {
        return Err(WsError::InvalidHeader("unsupported browser name"));
    }
    if !has_header(target, "user-agent") {
        if default_user_agent.is_empty() {
            return Err(WsError::InvalidHeader("default User-Agent is empty"));
        }
        target.add_header("User-Agent", default_user_agent)?;
    }
    if !has_header(target, "x-playwright-browser") {
        target.add_header("x-playwright-browser", browser_name)?;
    }
    if !has_header(target, "x-playwright-proxy") {
        target.add_header("x-playwright-proxy", "")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use pwright_transport_ws::{WsEndpoint, WsLimits, WsRequest};

    use super::*;

    fn request() -> WsRequest {
        let endpoint = WsEndpoint::parse("ws://example.test/ws").unwrap();
        let limits = WsLimits::new(1024, 1024, 100).unwrap();
        WsRequest::new(endpoint, limits)
    }

    #[test]
    fn explicit_user_agent_and_browser_header_override_defaults() {
        // Arrange
        let mut request = request();
        request.add_header("uSeR-aGeNt", "Playwright").unwrap();
        request
            .add_header("X-Playwright-Browser", "firefox")
            .unwrap();
        request.add_header("foo", "bar").unwrap();

        // Act
        append_server_connect_defaults(&mut request, "chromium", "ignored").unwrap();

        // Assert
        let headers = request
            .headers()
            .iter()
            .map(|header| (header.name(), header.value()))
            .collect::<Vec<_>>();
        assert!(headers.contains(&("uSeR-aGeNt", "Playwright")));
        assert!(headers.contains(&("X-Playwright-Browser", "firefox")));
        assert!(headers.contains(&("foo", "bar")));
        assert_eq!(headers.len(), 4);
        assert!(!needs_default_user_agent(&request));
    }

    #[test]
    fn default_headers_include_user_agent_browser_and_empty_proxy() {
        // Arrange
        let mut request = request();
        request.add_header("foo", "bar").unwrap();

        // Act
        append_server_connect_defaults(&mut request, "chromium", "Playwright/1.58.0").unwrap();

        // Assert
        let headers = request
            .headers()
            .iter()
            .map(|header| (header.name(), header.value()))
            .collect::<Vec<_>>();
        assert!(headers.contains(&("User-Agent", "Playwright/1.58.0")));
        assert!(headers.contains(&("x-playwright-browser", "chromium")));
        assert!(headers.contains(&("x-playwright-proxy", "")));
        assert!(headers.contains(&("foo", "bar")));
    }

    #[test]
    fn defaults_consume_the_same_bounded_header_slots() {
        // Arrange
        let mut request = request();
        for index in 0..63 {
            request.add_header(&format!("x-{index}"), "v").unwrap();
        }

        // Act
        let result = append_server_connect_defaults(&mut request, "chromium", "Playwright/1.58.0");

        // Assert
        assert!(matches!(
            result,
            Err(WsError::LimitExceeded {
                resource: "header count",
                ..
            })
        ));
    }

    #[test]
    fn http_discovery_defaults_share_budget_with_deferred_reserved_header() {
        // Arrange
        let mut headers = WsHeaders::default();
        headers
            .push_for_http_discovery("Connection", "Close")
            .unwrap();

        // Act
        append_server_connect_defaults_for_http_discovery(
            &mut headers,
            "chromium",
            "Playwright/1.58.0",
        )
        .unwrap();

        // Assert
        assert_eq!(
            headers
                .iter()
                .map(|header| (header.name(), header.value()))
                .collect::<Vec<_>>(),
            vec![
                ("Connection", "Close"),
                ("User-Agent", "Playwright/1.58.0"),
                ("x-playwright-browser", "chromium"),
                ("x-playwright-proxy", ""),
            ]
        );
    }
}
