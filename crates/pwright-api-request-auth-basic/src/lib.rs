//! Optional bounded HTTP Basic authorization for API requests.

use std::fmt;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use pwright_api_request::ApiAuthorizationProvider;
use url::Url;
use zeroize::Zeroizing;

pub const MAX_USERNAME_BYTES: usize = 4 * 1024;
pub const MAX_PASSWORD_BYTES: usize = 4 * 1024;
pub const MAX_ORIGIN_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicAuthorizationSend {
    Unauthorized,
    Always,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum BasicAuthorizationError {
    #[error("HTTP Basic username exceeds its configured limit")]
    UsernameTooLarge,
    #[error("HTTP Basic password exceeds its configured limit")]
    PasswordTooLarge,
    #[error("HTTP Basic origin exceeds its configured limit")]
    OriginTooLarge,
}

pub struct BasicAuthorizationProvider {
    value: Zeroizing<String>,
    origin: Option<String>,
    send: BasicAuthorizationSend,
}

impl BasicAuthorizationProvider {
    /// Create one bounded HTTP Basic credential policy.
    ///
    /// # Errors
    ///
    /// Returns an error when a credential exceeds its configured byte limit.
    pub fn new(
        username: &str,
        password: &str,
        origin: Option<&str>,
        send: BasicAuthorizationSend,
    ) -> Result<Self, BasicAuthorizationError> {
        if username.len() > MAX_USERNAME_BYTES {
            return Err(BasicAuthorizationError::UsernameTooLarge);
        }
        if password.len() > MAX_PASSWORD_BYTES {
            return Err(BasicAuthorizationError::PasswordTooLarge);
        }
        let origin = origin
            .map(|value| {
                if value.len() > MAX_ORIGIN_BYTES {
                    return Err(BasicAuthorizationError::OriginTooLarge);
                }
                Ok(value.to_ascii_lowercase())
            })
            .transpose()?;
        let joined = Zeroizing::new(format!("{username}:{password}"));
        let value = Zeroizing::new(format!("Basic {}", STANDARD.encode(joined.as_bytes())));
        Ok(Self {
            value,
            origin,
            send,
        })
    }

    fn matches(&self, url: &Url) -> bool {
        self.origin.as_ref().is_none_or(|origin| {
            url.origin()
                .ascii_serialization()
                .eq_ignore_ascii_case(origin)
        })
    }

    fn header(&self, url: &Url) -> Option<String> {
        self.matches(url).then(|| self.value.to_string())
    }
}

impl fmt::Debug for BasicAuthorizationProvider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BasicAuthorizationProvider")
            .field("value", &"[REDACTED]")
            .field("origin", &self.origin)
            .field("send", &self.send)
            .finish()
    }
}

impl ApiAuthorizationProvider for BasicAuthorizationProvider {
    fn preemptive_header(&self, url: &Url) -> Option<String> {
        (self.send == BasicAuthorizationSend::Always)
            .then(|| self.header(url))
            .flatten()
    }

    fn challenge_header(&self, url: &Url, www_authenticate: &str) -> Option<String> {
        www_authenticate
            .trim()
            .starts_with("Basic")
            .then(|| self.header(url))
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use pwright_api_request::ApiAuthorizationProvider;
    use url::Url;

    use super::{
        BasicAuthorizationError, BasicAuthorizationProvider, BasicAuthorizationSend,
        MAX_USERNAME_BYTES,
    };

    fn url(value: &str) -> Url {
        Url::parse(value).expect("test URL should parse")
    }

    #[test]
    fn challenge_mode_emits_exact_basic_value_only_after_basic_challenge() {
        // Arrange
        let provider = BasicAuthorizationProvider::new(
            "user",
            "pass",
            None,
            BasicAuthorizationSend::Unauthorized,
        )
        .expect("provider should build");
        let target = url("http://example.test/path");

        // Act
        let preemptive = provider.preemptive_header(&target);
        let unsupported = provider.challenge_header(&target, "Digest realm=test");
        let challenged = provider.challenge_header(&target, "  Basic realm=test");

        // Assert
        assert_eq!(preemptive, None);
        assert_eq!(unsupported, None);
        assert_eq!(challenged.as_deref(), Some("Basic dXNlcjpwYXNz"));
    }

    #[test]
    fn always_mode_emits_preemptively() {
        // Arrange
        let provider =
            BasicAuthorizationProvider::new("user", "pass", None, BasicAuthorizationSend::Always)
                .expect("provider should build");

        // Act
        let header = provider.preemptive_header(&url("https://example.test/"));

        // Assert
        assert_eq!(header.as_deref(), Some("Basic dXNlcjpwYXNz"));
    }

    #[test]
    fn origin_match_is_case_insensitive_but_scheme_host_and_port_exact() {
        // Arrange
        let provider = BasicAuthorizationProvider::new(
            "user",
            "pass",
            Some("HTTP://EXAMPLE.TEST:8080"),
            BasicAuthorizationSend::Always,
        )
        .expect("provider should build");

        // Act
        let exact = provider.preemptive_header(&url("http://example.test:8080/path"));
        let scheme = provider.preemptive_header(&url("https://example.test:8080/path"));
        let host = provider.preemptive_header(&url("http://other.test:8080/path"));
        let port = provider.preemptive_header(&url("http://example.test:8081/path"));
        let explicit_default_port = BasicAuthorizationProvider::new(
            "user",
            "pass",
            Some("http://example.test:80"),
            BasicAuthorizationSend::Always,
        )
        .expect("provider should build")
        .preemptive_header(&url("http://example.test/path"));

        // Assert
        assert!(exact.is_some());
        assert_eq!(scheme, None);
        assert_eq!(host, None);
        assert_eq!(port, None);
        assert_eq!(explicit_default_port, None);
    }

    #[test]
    fn credentials_are_redacted_from_debug_output() {
        // Arrange
        let provider = BasicAuthorizationProvider::new(
            "secret-user",
            "secret-pass",
            None,
            BasicAuthorizationSend::Always,
        )
        .expect("provider should build");

        // Act
        let debug = format!("{provider:?}");

        // Assert
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret-user"));
        assert!(!debug.contains("secret-pass"));
    }

    #[test]
    fn oversized_username_is_rejected_before_encoding() {
        // Arrange
        let username = "a".repeat(MAX_USERNAME_BYTES + 1);

        // Act
        let result = BasicAuthorizationProvider::new(
            &username,
            "pass",
            None,
            BasicAuthorizationSend::Unauthorized,
        );

        // Assert
        assert_eq!(
            result.expect_err("oversized username should fail"),
            BasicAuthorizationError::UsernameTooLarge
        );
    }

    #[test]
    fn arbitrary_origin_string_is_retained_for_exact_nonmatch() {
        // Arrange
        let origin = "http://example.test/path";

        // Act
        let provider = BasicAuthorizationProvider::new(
            "user",
            "pass",
            Some(origin),
            BasicAuthorizationSend::Unauthorized,
        )
        .expect("bounded origin should be retained");

        // Act
        let result = provider.challenge_header(&url("http://example.test/path"), "Basic");

        // Assert
        assert_eq!(result, None);
    }
}
