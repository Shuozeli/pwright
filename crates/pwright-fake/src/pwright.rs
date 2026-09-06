//! High-level fake pwright harness for downstream consumers.
//!
//! `FakeCdpClient` is the low-level in-memory CDP fake. `FakePwright` is a
//! small consumer-facing wrapper that gives downstream projects a stable
//! testing component without needing to construct CDP fakes directly.

use std::sync::Arc;

use pwright_cdp::CdpEvent;
use serde_json::Value;

use crate::FakeCdpClient;

/// Consumer-facing fake pwright harness.
///
/// This type intentionally stays thin. It owns an in-memory CDP client and
/// exposes the fake browser as an `Arc<FakeCdpClient>` so downstream adapters
/// can plug it into their pwright-facing boundary.
pub struct FakePwright {
    client: Arc<FakeCdpClient>,
}

impl FakePwright {
    /// Create a fake browser from static HTML.
    pub fn from_html(html: &str) -> Self {
        Self {
            client: Arc::new(FakeCdpClient::from_html(html)),
        }
    }

    /// Access the underlying fake CDP client for pwright bridge/page adapters.
    pub fn client(&self) -> Arc<FakeCdpClient> {
        self.client.clone()
    }

    /// Set a canned `Runtime.evaluate` response.
    pub fn set_evaluate_response(&self, response: Value) {
        self.client.set_evaluate_response(response);
    }

    /// Inject a CDP event, for example a network event.
    pub fn send_event(&self, event: CdpEvent) {
        self.client.send_event(event);
    }

    /// Return recorded CDP method names for behavior assertions.
    pub fn method_names(&self) -> Vec<String> {
        self.client.method_names()
    }

    /// Return recorded CDP calls for detailed assertions.
    pub fn calls(&self) -> Vec<(String, Vec<Value>)> {
        self.client.calls()
    }
}

#[cfg(test)]
mod tests {
    use pwright_cdp::CdpClient;

    use super::*;

    #[tokio::test]
    async fn fake_pwright_exposes_cdp_client_for_downstream_adapters() {
        // Arrange
        let fake = FakePwright::from_html("<button>Publish</button>");
        let client = fake.client();

        // Act
        client.page_navigate("https://example.com").await.unwrap();

        // Assert
        assert_eq!(fake.method_names(), vec!["Page.navigate"]);
    }
}
