//! Opt-in Playwright 1.58.0 CDP transport User-Agent policy.
//! Runtime observations are injected by the Node boundary; formatting and
//! case-insensitive header precedence stay in Rust.

use pwright_cdp::CdpHandshakeHeaders;
pub use pwright_playwright_connect_headers::{
    PLAYWRIGHT_COMPAT_VERSION, RuntimeInputs, format_user_agent,
};

use crate::HostError;

pub fn has_explicit_user_agent(headers: &[(String, String)]) -> bool {
    headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
}

pub fn validate_explicit_headers(headers: &[(String, String)]) -> Result<(), HostError> {
    CdpHandshakeHeaders::new(headers.to_vec())
        .map(|_| ())
        .map_err(|error| {
            if error.is_limit() {
                HostError::ResourceExhausted(error.to_string())
            } else {
                HostError::InvalidArgument(error.to_string())
            }
        })
}

pub fn with_default_user_agent(
    mut headers: Vec<(String, String)>,
    default_user_agent: &str,
) -> Vec<(String, String)> {
    if !has_explicit_user_agent(&headers) {
        headers.push(("User-Agent".into(), default_user_agent.into()));
    }
    headers
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_hdr_async;

    fn mac_inputs() -> RuntimeInputs {
        RuntimeInputs {
            platform: "darwin".into(),
            arch: "arm64".into(),
            release: "25.0.0".into(),
            mac_product_version: Some("26.0.1".into()),
            linux_os_release: None,
            node_version: Some((24, 12)),
            version_override: None,
            language_name: None,
            language_version: None,
            ci: false,
        }
    }

    #[test]
    fn default_uses_pinned_playwright_version_and_node_runtime() {
        // Arrange
        let inputs = mac_inputs();
        // Act
        let user_agent = format_user_agent(&inputs);
        // Assert
        assert_eq!(
            user_agent,
            "Playwright/1.58.0 (arm64; macOS 26.0) node/24.12"
        );
    }

    #[test]
    fn explicit_user_agent_wins_regardless_of_case_or_empty_value() {
        // Arrange
        let headers = vec![
            ("uSeR-aGeNt".into(), String::new()),
            ("foo".into(), "bar".into()),
        ];
        // Act
        let merged = with_default_user_agent(headers.clone(), "Playwright/default");
        // Assert
        assert!(has_explicit_user_agent(&headers));
        assert_eq!(merged, headers);
    }

    #[test]
    fn default_is_appended_only_when_absent() {
        // Arrange
        let headers = vec![("foo".into(), "bar".into())];
        // Act
        let merged = with_default_user_agent(headers, "Playwright/default");
        // Assert
        assert_eq!(
            merged,
            vec![
                ("foo".into(), "bar".into()),
                ("User-Agent".into(), "Playwright/default".into())
            ]
        );
    }

    #[test]
    fn synthesized_user_agent_consumes_the_last_header_slot() {
        // Arrange
        let headers = (0..32)
            .map(|index| (format!("x-{index}"), "v".into()))
            .collect::<Vec<_>>();
        assert!(validate_explicit_headers(&headers).is_ok());
        // Act
        let merged = with_default_user_agent(headers, "Playwright/default");
        let result = validate_explicit_headers(&merged);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    }

    #[test]
    fn language_version_empty_is_not_replaced_and_ci_is_appended() {
        // Arrange
        let mut inputs = mac_inputs();
        inputs.language_name = Some("python".into());
        inputs.language_version = Some(String::new());
        inputs.version_override = Some("1.39".into());
        inputs.ci = true;
        // Act
        let user_agent = format_user_agent(&inputs);
        // Assert
        assert_eq!(
            user_agent,
            "Playwright/1.39 (arm64; macOS 26.0) python/ CI/1"
        );
    }

    #[test]
    fn linux_release_and_missing_node_runtime_have_deterministic_results() {
        // Arrange
        let mut inputs = mac_inputs();
        inputs.platform = "linux".into();
        inputs.mac_product_version = None;
        inputs.linux_os_release = Some("ID=ubuntu\nVERSION_ID=\"24.04\"\n".into());
        // Act
        let linux = format_user_agent(&inputs);
        inputs.node_version = None;
        let fallback = format_user_agent(&inputs);
        // Assert
        assert_eq!(linux, "Playwright/1.58.0 (arm64; ubuntu 24.04) node/24.12");
        assert_eq!(fallback, "Playwright/unknown");
    }

    #[tokio::test]
    async fn merged_user_agent_reaches_http_discovery_and_websocket_upgrade() {
        // Arrange
        let (bind_ip, client_host) = match (
            std::env::var("TAILSCALE_IP"),
            std::env::var("TAILSCALE_HOST"),
        ) {
            (Ok(ip), Ok(host)) => {
                assert!(host.ends_with(".ts.net") && !host.contains('/') && !host.contains(':'));
                (
                    ip.parse::<Ipv4Addr>().expect("TAILSCALE_IP must be IPv4"),
                    host,
                )
            }
            (Err(std::env::VarError::NotPresent), Err(std::env::VarError::NotPresent)) => {
                // Portable CI without Tailscale still exercises the pure
                // formatter/limits. Do not open a wildcard or loopback server.
                eprintln!("skipping CDP UA receipt fixture: TAILSCALE_IP/HOST not set");
                return;
            }
            _ => panic!("TAILSCALE_IP and TAILSCALE_HOST must be set together"),
        };
        let listener = TcpListener::bind(SocketAddr::from((bind_ip, 0)))
            .await
            .expect("one-shot test listener should bind");
        let port = listener.local_addr().expect("bound address").port();
        let websocket_url = format!("ws://{client_host}:{port}/ws");
        let discovery_websocket = websocket_url.clone();
        let (upgrade_tx, upgrade_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("discovery request");
            let mut request = Vec::new();
            loop {
                let mut chunk = [0_u8; 1024];
                let count = stream.read(&mut chunk).await.expect("HTTP request read");
                assert!(count > 0 && request.len() + count <= 8192);
                request.extend_from_slice(&chunk[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let body = format!(r#"{{"webSocketDebuggerUrl":"{discovery_websocket}"}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("HTTP response");
            drop(stream);
            let (stream, _) = listener.accept().await.expect("WebSocket request");
            let mut upgrade_tx = Some(upgrade_tx);
            let websocket = accept_hdr_async(
                stream,
                move |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                      response| {
                    let user_agent = request
                        .headers()
                        .get("user-agent")
                        .map(|value| value.as_bytes().to_vec());
                    let foo = request
                        .headers()
                        .get("foo")
                        .map(|value| value.as_bytes().to_vec());
                    upgrade_tx
                        .take()
                        .expect("one upgrade")
                        .send((user_agent, foo))
                        .expect("upgrade receiver should remain active");
                    Ok(response)
                },
            )
            .await
            .expect("WebSocket handshake");
            drop(websocket);
            String::from_utf8(request).expect("HTTP headers should be UTF-8")
        });
        let default = format_user_agent(&mac_inputs());
        let headers = with_default_user_agent(vec![("foo".into(), "bar".into())], &default);

        // Act
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            crate::HostSession::connect_over_cdp_with_headers(
                &format!("http://{client_host}:{port}/browser/?foo=bar"),
                5_000,
                headers,
            ),
        )
        .await
        .expect("CDP attach should settle after peer closes");
        let discovery = tokio::time::timeout(Duration::from_secs(10), server)
            .await
            .expect("one-shot server should settle")
            .expect("one-shot server should retire");
        let upgrade = tokio::time::timeout(Duration::from_secs(10), upgrade_rx)
            .await
            .expect("upgrade receipt should settle")
            .expect("upgrade receipt should arrive");

        // Assert
        assert!(
            result.is_err(),
            "fixture intentionally sends no CDP response"
        );
        assert!(discovery.starts_with("GET /browser/json/version/?foo=bar HTTP/1.1\r\n"));
        let discovery_lower = discovery.to_ascii_lowercase();
        assert!(discovery_lower.contains(&format!(
            "\r\nuser-agent: {}\r\n",
            default.to_ascii_lowercase()
        )));
        assert!(discovery_lower.contains("\r\nfoo: bar\r\n"));
        assert_eq!(upgrade.0, Some(default.into_bytes()));
        assert_eq!(upgrade.1, Some(b"bar".to_vec()));
    }
}
