use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

use reqwest::header::{LOCATION, USER_AGENT};

use crate::{
    DiscoveryRequest, DiscoveryResponse, FetchError, HttpDiscoveryFetcher, MAX_DISCOVERY_REDIRECTS,
};
use pwright_transport_ws::MAX_ENDPOINT_BYTES;

/// Explicit live discovery transport with bounded HTTP redirects.
pub struct ReqwestDiscoveryFetcher {
    client: reqwest::Client,
}

impl ReqwestDiscoveryFetcher {
    pub fn new() -> Result<Self, FetchError> {
        let client = reqwest::Client::builder()
            // Follow redirects ourselves because Playwright follows every 3xx
            // with Location, including 300 and 304. The URL query may contain
            // credentials, so no redirect may synthesize a Referer.
            .referer(false)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| FetchError::Transport)?;
        Ok(Self { client })
    }
}

fn redirect_target(
    status: u16,
    current: &reqwest::Url,
    location: Option<&str>,
    followed: usize,
) -> Option<reqwest::Url> {
    if !(300..400).contains(&status) || followed >= MAX_DISCOVERY_REDIRECTS {
        return None;
    }
    let location =
        location.filter(|value| !value.is_empty() && value.len() <= MAX_ENDPOINT_BYTES)?;
    let target = current.join(location).ok()?;
    if !matches!(target.scheme(), "http" | "https")
        || !target.username().is_empty()
        || target.password().is_some()
        || target.fragment().is_some()
        || target.as_str().len() > MAX_ENDPOINT_BYTES
    {
        return None;
    }
    Some(target)
}

fn remaining_timeout(limit: Duration, elapsed: Duration) -> Result<Duration, FetchError> {
    let remaining = limit.checked_sub(elapsed).ok_or(FetchError::Timeout)?;
    if remaining.is_zero() {
        return Err(FetchError::Timeout);
    }
    Ok(remaining)
}

impl HttpDiscoveryFetcher for ReqwestDiscoveryFetcher {
    fn fetch<'a>(
        &'a self,
        request: &'a DiscoveryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<DiscoveryResponse, FetchError>> + Send + 'a>> {
        Box::pin(async move {
            let mut url = request.url().clone();
            let mut followed = 0;
            let limit = Duration::from_millis(u64::from(request.limits().timeout_ms()));
            let started = Instant::now();
            loop {
                let remaining = remaining_timeout(limit, started.elapsed())?;
                let mut response = self
                    .client
                    .get(url.clone())
                    .header(USER_AGENT, request.user_agent())
                    .timeout(remaining)
                    .send()
                    .await
                    .map_err(map_reqwest_error)?;
                let status = response.status().as_u16();
                let location = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|v| v.to_str().ok());
                if let Some(target) = redirect_target(status, &url, location, followed) {
                    url = target;
                    followed += 1;
                    continue;
                }
                if status != 200 {
                    return Ok(DiscoveryResponse::new(status, Vec::new()));
                }
                let mut body = Vec::new();
                while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
                    let remaining = request.limits().max_body_bytes() - body.len();
                    if chunk.len() > remaining {
                        return Err(FetchError::BodyTooLarge);
                    }
                    body.extend_from_slice(&chunk);
                }
                return Ok(DiscoveryResponse::new(status, body));
            }
        })
    }
}

fn map_reqwest_error(error: reqwest::Error) -> FetchError {
    if error.is_timeout() {
        FetchError::Timeout
    } else {
        // `reqwest::Error` can carry the full URL, including query secrets.
        FetchError::Transport
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redirect_policy_allows_every_3xx_with_location_and_exactly_five_hops() {
        // Arrange
        let url = reqwest::Url::parse("https://server.example/json?token=secret").unwrap();

        // Act
        let statuses = [299, 300, 301, 302, 303, 304, 305, 306, 307, 308, 399, 400]
            .map(|status| redirect_target(status, &url, Some("/next"), 0).is_some());
        let fifth = redirect_target(302, &url, Some("/next"), MAX_DISCOVERY_REDIRECTS - 1);
        let sixth = redirect_target(302, &url, Some("/next"), MAX_DISCOVERY_REDIRECTS);

        // Assert
        assert_eq!(
            statuses,
            [
                false, true, true, true, true, true, true, true, true, true, true, false
            ]
        );
        assert_eq!(fifth.unwrap().as_str(), "https://server.example/next");
        assert!(sixth.is_none());
    }

    #[test]
    fn redirect_policy_rejects_unsafe_or_overlong_targets() {
        // Arrange
        let original = reqwest::Url::parse("http://server.example/json").unwrap();
        let overlong = format!("/{}", "x".repeat(MAX_ENDPOINT_BYTES));

        // Act
        let accepted = [
            redirect_target(302, &original, Some("/next"), 0).is_some(),
            redirect_target(
                302,
                &original,
                Some("http://user:secret@server.example/json"),
                0,
            )
            .is_some(),
            redirect_target(302, &original, Some("/json#secret"), 0).is_some(),
            redirect_target(302, &original, Some("file:///tmp/json"), 0).is_some(),
            redirect_target(302, &original, Some(&overlong), 0).is_some(),
            redirect_target(302, &original, None, 0).is_some(),
            redirect_target(302, &original, Some(""), 0).is_some(),
        ];

        // Assert
        assert_eq!(accepted, [true, false, false, false, false, false, false]);
    }

    #[test]
    fn redirect_timeout_is_cumulative_across_requests() {
        // Arrange
        let limit = Duration::from_millis(100);

        // Act
        let first = remaining_timeout(limit, Duration::from_millis(30));
        let second = remaining_timeout(limit, Duration::from_millis(80));
        let exhausted = remaining_timeout(limit, Duration::from_millis(100));

        // Assert
        assert_eq!(first, Ok(Duration::from_millis(70)));
        assert_eq!(second, Ok(Duration::from_millis(20)));
        assert_eq!(exhausted, Err(FetchError::Timeout));
    }
}
