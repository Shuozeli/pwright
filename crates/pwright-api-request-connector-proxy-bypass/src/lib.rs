//! Optional Playwright-compatible proxy bypass selection for API requests.
//!
//! This leaf owns only target selection. It composes one direct connector with
//! any independently selected proxy connector, so HTTP CONNECT and SOCKS do
//! not duplicate bypass policy or depend on each other.

use std::sync::Arc;

use async_trait::async_trait;
use pwright_api_request_transport::{
    ApiConnection, ApiConnectionError, ApiConnectionErrorKind, ApiConnectionTarget, ApiConnector,
};
use tokio_util::sync::CancellationToken;

const MAX_BYPASS_BYTES: usize = 64 * 1024;
const MAX_BYPASS_ENTRIES: usize = 4 * 1024;

/// Selects the direct connector when the request hostname matches one of the
/// comma-separated Playwright API-request bypass suffixes.
pub struct ProxyBypassConnector {
    direct: Arc<dyn ApiConnector>,
    proxied: Arc<dyn ApiConnector>,
    domains: Box<[Box<str>]>,
}

impl ProxyBypassConnector {
    /// Build one bounded bypass selector.
    ///
    /// Each entry is trimmed and receives a leading dot when absent. Matching
    /// then compares `.<request hostname>` by suffix, exactly like Playwright's
    /// API-request proxy-agent selection.
    ///
    /// # Errors
    ///
    /// Returns a proxy configuration error when the bypass input exceeds its
    /// byte or entry bound.
    pub fn new(
        direct: Arc<dyn ApiConnector>,
        proxied: Arc<dyn ApiConnector>,
        bypass: &str,
    ) -> Result<Self, ApiConnectionError> {
        if bypass.len() > MAX_BYPASS_BYTES {
            return Err(proxy_error(format!(
                "proxy bypass exceeds {MAX_BYPASS_BYTES} UTF-8 bytes"
            )));
        }
        let entry_count = bypass.split(',').count();
        if entry_count > MAX_BYPASS_ENTRIES {
            return Err(proxy_error(format!(
                "proxy bypass exceeds {MAX_BYPASS_ENTRIES} entries"
            )));
        }
        let mut domains = Vec::new();
        domains
            .try_reserve_exact(entry_count)
            .map_err(|error| proxy_error(error.to_string()))?;
        for value in bypass.split(',') {
            let value = value.trim();
            let domain = if value.starts_with('.') {
                value.to_owned()
            } else {
                format!(".{value}")
            };
            domains.push(domain.into_boxed_str());
        }
        Ok(Self {
            direct,
            proxied,
            domains: domains.into_boxed_slice(),
        })
    }

    fn should_bypass(&self, target: ApiConnectionTarget<'_>) -> bool {
        let Some(hostname) = target.url.host_str() else {
            return false;
        };
        let hostname = format!(".{hostname}");
        self.domains
            .iter()
            .any(|domain| hostname.ends_with(domain.as_ref()))
    }
}

impl std::fmt::Debug for ProxyBypassConnector {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProxyBypassConnector")
            .field("domains", &self.domains)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ApiConnector for ProxyBypassConnector {
    async fn connect(
        &self,
        target: ApiConnectionTarget<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
        let connector = if self.should_bypass(target) {
            &self.direct
        } else {
            &self.proxied
        };
        connector.connect(target, cancellation).await
    }
}

fn proxy_error(message: impl Into<String>) -> ApiConnectionError {
    ApiConnectionError::new(ApiConnectionErrorKind::Proxy, message)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use pwright_api_request_transport::{
        ApiConnection, ApiConnectionError, ApiConnectionTarget, ApiConnector,
    };
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::{MAX_BYPASS_BYTES, MAX_BYPASS_ENTRIES, ProxyBypassConnector};

    struct RecordingConnector {
        name: &'static str,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl ApiConnector for RecordingConnector {
        async fn connect(
            &self,
            _target: ApiConnectionTarget<'_>,
            _cancellation: &CancellationToken,
        ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
            self.calls.lock().expect("call log lock").push(self.name);
            Ok(Box::new(EmptyConnection))
        }
    }

    struct EmptyConnection;

    #[async_trait]
    impl ApiConnection for EmptyConnection {
        async fn write_all(
            &mut self,
            _bytes: &[u8],
            _cancellation: &CancellationToken,
        ) -> Result<(), ApiConnectionError> {
            Ok(())
        }

        async fn read(
            &mut self,
            _output: &mut [u8],
            _cancellation: &CancellationToken,
        ) -> Result<usize, ApiConnectionError> {
            Ok(0)
        }
    }

    fn fixture(bypass: &str) -> (ProxyBypassConnector, Arc<Mutex<Vec<&'static str>>>) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let direct: Arc<dyn ApiConnector> = Arc::new(RecordingConnector {
            name: "direct",
            calls: calls.clone(),
        });
        let proxied: Arc<dyn ApiConnector> = Arc::new(RecordingConnector {
            name: "proxied",
            calls: calls.clone(),
        });
        (
            ProxyBypassConnector::new(direct, proxied, bypass).expect("valid bypass"),
            calls,
        )
    }

    async fn connect(connector: &ProxyBypassConnector, url: &str) {
        let url = Url::parse(url).expect("target URL");
        connector
            .connect(
                ApiConnectionTarget {
                    url: &url,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await
            .expect("connector selection should succeed");
    }

    #[tokio::test]
    async fn exact_hostname_uses_the_direct_connector() {
        // Arrange
        let (connector, calls) = fixture("one.example.test");

        // Act
        connect(&connector, "http://one.example.test/resource").await;

        // Assert
        assert_eq!(calls.lock().expect("call log lock").as_slice(), ["direct"]);
    }

    #[tokio::test]
    async fn leading_dot_matches_subdomains_after_trimming() {
        // Arrange
        let (connector, calls) = fixture(" other.test, .another.test ");

        // Act
        connect(&connector, "http://foo.is.the.another.test/resource").await;

        // Assert
        assert_eq!(calls.lock().expect("call log lock").as_slice(), ["direct"]);
    }

    #[tokio::test]
    async fn hostname_suffix_requires_a_label_boundary() {
        // Arrange
        let (connector, calls) = fixture("example.test");

        // Act
        connect(&connector, "http://notexample.test/resource").await;

        // Assert
        assert_eq!(calls.lock().expect("call log lock").as_slice(), ["proxied"]);
    }

    #[tokio::test]
    async fn nonmatching_hostname_uses_the_proxy_connector() {
        // Arrange
        let (connector, calls) = fixture("one.example.test, two.example.test");

        // Act
        connect(&connector, "http://three.example.test/resource").await;

        // Assert
        assert_eq!(calls.lock().expect("call log lock").as_slice(), ["proxied"]);
    }

    #[test]
    fn constructor_rejects_oversized_input_before_retaining_it() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let direct: Arc<dyn ApiConnector> = Arc::new(RecordingConnector {
            name: "direct",
            calls: calls.clone(),
        });
        let proxied: Arc<dyn ApiConnector> = Arc::new(RecordingConnector {
            name: "proxied",
            calls,
        });
        let bypass = "x".repeat(MAX_BYPASS_BYTES + 1);

        // Act
        let result = ProxyBypassConnector::new(direct, proxied, &bypass);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn constructor_rejects_too_many_entries() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let direct: Arc<dyn ApiConnector> = Arc::new(RecordingConnector {
            name: "direct",
            calls: calls.clone(),
        });
        let proxied: Arc<dyn ApiConnector> = Arc::new(RecordingConnector {
            name: "proxied",
            calls,
        });
        let bypass = std::iter::repeat_n("", MAX_BYPASS_ENTRIES + 1)
            .collect::<Vec<_>>()
            .join(",");

        // Act
        let result = ProxyBypassConnector::new(direct, proxied, &bypass);

        // Assert
        assert!(result.is_err());
    }
}
