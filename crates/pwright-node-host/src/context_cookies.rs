//! Optional BrowserContext cookie capability.
//!
//! Context routing, Playwright-compatible validation/filtering and Chromium
//! Storage-domain commands stay in Rust. The capability retains no cookie
//! mirror: Chromium remains the sole source of truth.

use pwright_cdp::CdpSession;
use pwright_cdp::domains::storage::StorageCookie as CdpCookie;
use pwright_cdp::generated::network::{CookieParam, CookiePartitionKey, CookieSameSite};
use pwright_core::ObjectId;
use regex::{Regex, RegexBuilder};
use url::Url;

use crate::{HostError, HostSession};

pub const MAX_COOKIES: usize = 1024;
pub const MAX_COOKIE_FIELD_BYTES: usize = 16 * 1024;
pub const MAX_COOKIE_BATCH_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_COOKIE_URLS: usize = 256;
pub const MAX_COOKIE_URL_BYTES: usize = 64 * 1024;
pub const MAX_COOKIE_URL_BATCH_BYTES: usize = 1024 * 1024;
pub const MAX_FILTER_PATTERN_BYTES: usize = 16 * 1024;
pub const FFI_ALLOWANCE: u64 = 2 * MAX_COOKIE_BATCH_BYTES as u64 + 2 * 1024 * 1024;
const MAX_EXPIRES: f64 = 253_402_300_799.0;

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserContextCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: String,
    pub partition_key: Option<String>,
    pub partition_has_cross_site_ancestor: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrowserContextCookieInput {
    pub name: String,
    pub value: String,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<f64>,
    pub http_only: Option<bool>,
    pub secure: Option<bool>,
    pub same_site: Option<String>,
    pub partition_key: Option<String>,
    pub partition_has_cross_site_ancestor: Option<bool>,
}

#[derive(Clone, Debug)]
pub enum CookieStringMatcher {
    Exact(String),
    Regex(Regex),
}

impl CookieStringMatcher {
    pub fn exact(value: String) -> Result<Self, HostError> {
        bounded_field(&value, MAX_COOKIE_FIELD_BYTES, "cookie clear filter")?;
        Ok(Self::Exact(value))
    }

    pub fn regex(source: String, flags: &str) -> Result<Self, HostError> {
        bounded_field(&source, MAX_FILTER_PATTERN_BYTES, "cookie clear regex")?;
        if flags.len() > 16 {
            return Err(HostError::InvalidArgument(
                "cookie clear regex flags exceed 16 UTF-8 bytes".into(),
            ));
        }
        let mut builder = RegexBuilder::new(&source);
        for flag in flags.chars() {
            match flag {
                'i' => {
                    builder.case_insensitive(true);
                }
                'm' => {
                    builder.multi_line(true);
                }
                's' => {
                    builder.dot_matches_new_line(true);
                }
                'x' => {
                    builder.ignore_whitespace(true);
                }
                // Python's ASCII/Unicode markers do not change ordinary
                // literal cookie filters.
                'a' | 'u' => {}
                _ => {
                    return Err(HostError::InvalidArgument(
                        "unsupported cookie clear regex flag".into(),
                    ));
                }
            }
        }
        builder
            .size_limit(MAX_COOKIE_BATCH_BYTES)
            .build()
            .map(Self::Regex)
            .map_err(|error| {
                HostError::InvalidArgument(format!("invalid cookie clear regex: {error}"))
            })
    }

    fn matches(&self, value: &str) -> bool {
        match self {
            // Playwright's filtered clear helper treats a falsy string as an
            // omitted matcher, while a RegExp with an empty source remains a
            // real matcher. Preserve that public edge exactly.
            Self::Exact(expected) => expected.is_empty() || value == expected,
            Self::Regex(pattern) => pattern.is_match(value),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BrowserContextCookieClearFilter {
    pub name: Option<CookieStringMatcher>,
    pub domain: Option<CookieStringMatcher>,
    pub path: Option<CookieStringMatcher>,
}

impl BrowserContextCookieClearFilter {
    fn is_empty(&self) -> bool {
        self.name.is_none() && self.domain.is_none() && self.path.is_none()
    }

    fn matches(&self, cookie: &BrowserContextCookie) -> bool {
        self.name
            .as_ref()
            .is_none_or(|matcher| matcher.matches(&cookie.name))
            && self
                .domain
                .as_ref()
                .is_none_or(|matcher| matcher.matches(&cookie.domain))
            && self
                .path
                .as_ref()
                .is_none_or(|matcher| matcher.matches(&cookie.path))
    }
}

fn bounded_field(value: &str, maximum: usize, field: &str) -> Result<(), HostError> {
    if value.len() > maximum {
        return Err(HostError::ResourceExhausted(format!(
            "{field} exceeds {maximum} UTF-8 bytes"
        )));
    }
    Ok(())
}

fn checked_add(
    total: &mut usize,
    value: usize,
    maximum: usize,
    field: &str,
) -> Result<(), HostError> {
    *total = total
        .checked_add(value)
        .ok_or_else(|| HostError::ResourceExhausted(format!("{field} size overflow")))?;
    if *total > maximum {
        return Err(HostError::ResourceExhausted(format!(
            "{field} exceeds {maximum} bytes"
        )));
    }
    Ok(())
}

fn validate_urls(urls: &[String]) -> Result<Vec<Url>, HostError> {
    if urls.len() > MAX_COOKIE_URLS {
        return Err(HostError::ResourceExhausted(format!(
            "cookie URL count exceeds {MAX_COOKIE_URLS}"
        )));
    }
    let mut total = 0;
    let mut parsed = Vec::new();
    parsed
        .try_reserve_exact(urls.len())
        .map_err(|_| HostError::ResourceExhausted("cookie URL allocation failed".into()))?;
    for value in urls {
        bounded_field(value, MAX_COOKIE_URL_BYTES, "cookie URL")?;
        checked_add(
            &mut total,
            value.len(),
            MAX_COOKIE_URL_BATCH_BYTES,
            "cookie URL batch",
        )?;
        parsed.push(
            Url::parse(value)
                .map_err(|_| HostError::InvalidArgument(format!("invalid cookie URL: {value}")))?,
        );
    }
    Ok(parsed)
}

fn same_site(value: Option<&CookieSameSite>) -> &'static str {
    match value {
        Some(CookieSameSite::Strict) => "Strict",
        Some(CookieSameSite::None) => "None",
        Some(CookieSameSite::Lax) | None => "Lax",
    }
}

fn parse_same_site(value: Option<&str>) -> Result<Option<CookieSameSite>, HostError> {
    value
        .map(|value| match value {
            "Strict" => Ok(CookieSameSite::Strict),
            "Lax" => Ok(CookieSameSite::Lax),
            "None" => Ok(CookieSameSite::None),
            _ => Err(HostError::InvalidArgument(
                "cookie sameSite must be Strict, Lax or None".into(),
            )),
        })
        .transpose()
}

fn cookie_payload_bytes(cookie: &BrowserContextCookie) -> Result<usize, HostError> {
    let mut bytes = 0;
    for value in [
        cookie.name.as_str(),
        cookie.value.as_str(),
        cookie.domain.as_str(),
        cookie.path.as_str(),
        cookie.same_site.as_str(),
    ] {
        bounded_field(value, MAX_COOKIE_FIELD_BYTES, "cookie field")?;
        checked_add(
            &mut bytes,
            value.len(),
            MAX_COOKIE_BATCH_BYTES,
            "cookie batch",
        )?;
    }
    if let Some(value) = cookie.partition_key.as_deref() {
        bounded_field(value, MAX_COOKIE_URL_BYTES, "cookie partition key")?;
        checked_add(
            &mut bytes,
            value.len(),
            MAX_COOKIE_BATCH_BYTES,
            "cookie batch",
        )?;
    }
    Ok(bytes)
}

fn convert_cookies(cookies: Vec<CdpCookie>) -> Result<Vec<BrowserContextCookie>, HostError> {
    if cookies.len() > MAX_COOKIES {
        return Err(HostError::ResourceExhausted(format!(
            "cookie result count exceeds {MAX_COOKIES}"
        )));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(cookies.len())
        .map_err(|_| HostError::ResourceExhausted("cookie result allocation failed".into()))?;
    let mut bytes = 0;
    for cookie in cookies {
        if !cookie.expires.is_finite() {
            return Err(HostError::Backend(
                "Chromium returned a non-finite cookie expiry".into(),
            ));
        }
        let cookie = BrowserContextCookie {
            name: cookie.name,
            value: cookie.value,
            domain: cookie.domain,
            path: cookie.path,
            expires: cookie.expires,
            http_only: cookie.http_only,
            secure: cookie.secure,
            same_site: same_site(cookie.same_site.as_ref()).to_owned(),
            partition_has_cross_site_ancestor: cookie
                .partition_key
                .as_ref()
                .map(|key| key.has_cross_site_ancestor),
            partition_key: cookie.partition_key.map(|key| key.top_level_site),
        };
        checked_add(
            &mut bytes,
            cookie_payload_bytes(&cookie)?,
            MAX_COOKIE_BATCH_BYTES,
            "cookie result batch",
        )?;
        output.push(cookie);
    }
    Ok(output)
}

fn cookie_matches_url(cookie: &BrowserContextCookie, url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let domain = cookie.domain.strip_prefix('.').unwrap_or(&cookie.domain);
    let domain_matches = host == domain
        || (host.len() > domain.len()
            && host.ends_with(domain)
            && host.as_bytes()[host.len() - domain.len() - 1] == b'.');
    if !domain_matches || !url.path().starts_with(&cookie.path) {
        return false;
    }
    !cookie.secure || url.scheme() == "https" || host == "localhost" || host.ends_with(".localhost")
}

fn filter_urls(cookies: Vec<BrowserContextCookie>, urls: &[Url]) -> Vec<BrowserContextCookie> {
    if urls.is_empty() {
        return cookies;
    }
    cookies
        .into_iter()
        .filter(|cookie| urls.iter().any(|url| cookie_matches_url(cookie, url)))
        .collect()
}

fn validate_expiry(value: Option<f64>) -> Result<(), HostError> {
    let Some(value) = value else {
        return Ok(());
    };
    if !value.is_finite() || (value < 0.0 && value != -1.0) || value > MAX_EXPIRES {
        return Err(HostError::InvalidArgument(
            "cookie expires must be -1 or a non-negative Unix timestamp no greater than 253402300799"
                .into(),
        ));
    }
    Ok(())
}

fn input_to_cdp(input: BrowserContextCookieInput) -> Result<CookieParam, HostError> {
    let mut bytes = 0;
    for (field, value, maximum) in [
        (
            "cookie name",
            Some(input.name.as_str()),
            MAX_COOKIE_FIELD_BYTES,
        ),
        (
            "cookie value",
            Some(input.value.as_str()),
            MAX_COOKIE_FIELD_BYTES,
        ),
        ("cookie URL", input.url.as_deref(), MAX_COOKIE_URL_BYTES),
        (
            "cookie domain",
            input.domain.as_deref(),
            MAX_COOKIE_FIELD_BYTES,
        ),
        ("cookie path", input.path.as_deref(), MAX_COOKIE_FIELD_BYTES),
        (
            "cookie partition key",
            input.partition_key.as_deref(),
            MAX_COOKIE_URL_BYTES,
        ),
    ] {
        if let Some(value) = value {
            bounded_field(value, maximum, field)?;
            checked_add(
                &mut bytes,
                value.len(),
                MAX_COOKIE_BATCH_BYTES,
                "cookie batch",
            )?;
        }
    }
    let has_url = input.url.as_ref().is_some_and(|value| !value.is_empty());
    let has_domain = input.domain.as_ref().is_some_and(|value| !value.is_empty());
    let has_path = input.path.as_ref().is_some_and(|value| !value.is_empty());
    if !has_url && (!has_domain || !has_path) {
        return Err(HostError::InvalidArgument(
            "Cookie should have a url or a domain/path pair".into(),
        ));
    }
    if has_url && has_domain {
        return Err(HostError::InvalidArgument(
            "Cookie should have either url or domain".into(),
        ));
    }
    if has_url && has_path {
        return Err(HostError::InvalidArgument(
            "Cookie should have either url or path".into(),
        ));
    }
    validate_expiry(input.expires)?;
    let same_site = parse_same_site(input.same_site.as_deref())?;
    let mut domain = input.domain;
    let mut path = input.path;
    let mut secure = input.secure;
    if let Some(url) = input.url.as_deref().filter(|url| !url.is_empty()) {
        if url == "about:blank" {
            return Err(HostError::InvalidArgument(format!(
                "Blank page can not have cookie \"{}\"",
                input.name
            )));
        }
        if url.starts_with("data:") {
            return Err(HostError::InvalidArgument(format!(
                "Data URL page can not have cookie \"{}\"",
                input.name
            )));
        }
        let parsed = Url::parse(url)
            .map_err(|_| HostError::InvalidArgument(format!("invalid cookie URL: {url}")))?;
        if parsed.host_str().is_none() {
            return Err(HostError::InvalidArgument(format!(
                "invalid cookie URL: {url}"
            )));
        }
        domain = parsed.host_str().map(str::to_owned);
        let pathname = parsed.path();
        let directory_end = pathname.rfind('/').unwrap_or(0);
        path = Some(pathname[..=directory_end].to_owned());
        secure = Some(parsed.scheme() == "https");
    }
    Ok(CookieParam {
        name: input.name,
        value: input.value,
        url: input.url,
        domain,
        path,
        secure,
        http_only: input.http_only,
        same_site,
        expires: input.expires,
        priority: None,
        same_party: None,
        source_scheme: None,
        source_port: None,
        partition_key: input
            .partition_key
            .map(|top_level_site| CookiePartitionKey {
                top_level_site,
                has_cross_site_ancestor: input.partition_has_cross_site_ancestor.unwrap_or(true),
            }),
    })
}

fn inputs_to_cdp(cookies: Vec<BrowserContextCookieInput>) -> Result<Vec<CookieParam>, HostError> {
    if cookies.len() > MAX_COOKIES {
        return Err(HostError::ResourceExhausted(format!(
            "cookie input count exceeds {MAX_COOKIES}"
        )));
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(cookies.len())
        .map_err(|_| HostError::ResourceExhausted("cookie input allocation failed".into()))?;
    let mut bytes = 0;
    for cookie in cookies {
        let cookie = input_to_cdp(cookie)?;
        let encoded = serde_json::to_vec(&cookie).map_err(HostError::backend)?;
        checked_add(
            &mut bytes,
            encoded.len(),
            MAX_COOKIE_BATCH_BYTES,
            "cookie input batch",
        )?;
        output.push(cookie);
    }
    Ok(output)
}

fn expired_cookie(cookie: BrowserContextCookie) -> BrowserContextCookieInput {
    BrowserContextCookieInput {
        name: cookie.name,
        value: String::new(),
        domain: Some(cookie.domain),
        path: Some(cookie.path),
        expires: Some(0.0),
        http_only: Some(cookie.http_only),
        secure: Some(cookie.secure),
        same_site: Some(cookie.same_site),
        partition_key: cookie.partition_key,
        partition_has_cross_site_ancestor: cookie.partition_has_cross_site_ancestor,
        url: None,
    }
}

impl HostSession {
    fn context_cookie_route(&self, context: ObjectId) -> Result<(CdpSession, String), HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let live = state.as_ref().ok_or(HostError::TargetClosed)?;
        let topology = live.composition.topology().map_err(HostError::backend)?;
        let route = topology
            .topology()
            .browser_context_route(context)
            .ok_or(HostError::TargetClosed)?;
        Ok((
            CdpSession::browser(live.composition.connection().clone()),
            route.browser_context_id().to_owned(),
        ))
    }

    pub async fn context_cookies(
        &self,
        context_id: &str,
        urls: Vec<String>,
    ) -> Result<Vec<BrowserContextCookie>, HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let urls = validate_urls(&urls)?;
        let (session, native_context) = self.context_cookie_route(context)?;
        let cookies = session
            .storage_get_cookies(&native_context)
            .await
            .map_err(HostError::backend)?;
        convert_cookies(cookies).map(|cookies| filter_urls(cookies, &urls))
    }

    pub async fn context_add_cookies(
        &self,
        context_id: &str,
        cookies: Vec<BrowserContextCookieInput>,
    ) -> Result<(), HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let cookies = inputs_to_cdp(cookies)?;
        let (session, native_context) = self.context_cookie_route(context)?;
        session
            .storage_set_cookies(&native_context, &cookies)
            .await
            .map_err(HostError::backend)
    }

    pub async fn context_clear_cookies(
        &self,
        context_id: &str,
        filter: BrowserContextCookieClearFilter,
    ) -> Result<(), HostError> {
        let (context, _call, _operation) = self.admit_operation(context_id)?;
        let (session, native_context) = self.context_cookie_route(context)?;
        if filter.is_empty() {
            return session
                .storage_clear_cookies(&native_context)
                .await
                .map_err(HostError::backend);
        }
        let cookies = session
            .storage_get_cookies(&native_context)
            .await
            .map_err(HostError::backend)?;
        let expired = convert_cookies(cookies)?
            .into_iter()
            .filter(|cookie| filter.matches(cookie))
            .map(expired_cookie)
            .collect::<Vec<_>>();
        if expired.is_empty() {
            return Ok(());
        }
        let expired = inputs_to_cdp(expired)?;
        session
            .storage_set_cookies(&native_context, &expired)
            .await
            .map_err(HostError::backend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cookie(domain: &str, path: &str, secure: bool) -> BrowserContextCookie {
        BrowserContextCookie {
            name: "session".into(),
            value: "value".into(),
            domain: domain.into(),
            path: path.into(),
            expires: -1.0,
            http_only: false,
            secure,
            same_site: "Lax".into(),
            partition_key: None,
            partition_has_cross_site_ancestor: None,
        }
    }

    #[test]
    fn url_filter_matches_domain_path_and_secure_semantics() {
        // Arrange
        let matching = cookie(".example.test", "/api", true);
        let wrong_path = cookie("example.test", "/other", false);
        let url = Url::parse("https://www.example.test/api/value").unwrap();

        // Act
        let values = filter_urls(vec![matching.clone(), wrong_path], &[url]);

        // Assert
        assert_eq!(values, vec![matching]);
    }

    #[test]
    fn localhost_allows_secure_cookie_filtering_over_http() {
        // Arrange
        let localhost = cookie("localhost", "/", true);
        let url = Url::parse("http://localhost/value").unwrap();

        // Act
        let values = filter_urls(vec![localhost.clone()], &[url]);

        // Assert
        assert_eq!(values, vec![localhost]);
    }

    #[test]
    fn cookie_input_requires_url_or_domain_path_pair() {
        // Arrange
        let input = BrowserContextCookieInput {
            name: "session".into(),
            value: "value".into(),
            ..Default::default()
        };

        // Act
        let error = input_to_cdp(input).unwrap_err();

        // Assert
        assert!(error.to_string().contains("url or a domain/path pair"));
    }

    #[test]
    fn cookie_input_preserves_url_for_chromium_defaulting() {
        // Arrange
        let input = BrowserContextCookieInput {
            name: "session".into(),
            value: "value".into(),
            url: Some("https://example.test/api/value".into()),
            same_site: Some("Strict".into()),
            ..Default::default()
        };

        // Act
        let output = input_to_cdp(input).unwrap();

        // Assert
        assert_eq!(
            output.url.as_deref(),
            Some("https://example.test/api/value")
        );
        assert_eq!(output.same_site, Some(CookieSameSite::Strict));
        assert_eq!(output.domain.as_deref(), Some("example.test"));
        assert_eq!(output.path.as_deref(), Some("/api/"));
        assert_eq!(output.secure, Some(true));
    }

    #[test]
    fn empty_url_uses_the_domain_path_pair_like_playwright() {
        // Arrange
        let input = BrowserContextCookieInput {
            name: "session".into(),
            value: "value".into(),
            url: Some(String::new()),
            domain: Some("example.test".into()),
            path: Some("/".into()),
            ..Default::default()
        };

        // Act
        let output = input_to_cdp(input).unwrap();

        // Assert
        assert_eq!(output.url.as_deref(), Some(""));
        assert_eq!(output.domain.as_deref(), Some("example.test"));
        assert_eq!(output.path.as_deref(), Some("/"));
    }

    #[test]
    fn clear_filter_supports_exact_and_bounded_regex_matchers() {
        // Arrange
        let filter = BrowserContextCookieClearFilter {
            name: Some(CookieStringMatcher::regex("^sess".into(), "i").unwrap()),
            domain: Some(CookieStringMatcher::exact("example.test".into()).unwrap()),
            path: None,
        };
        let value = cookie("example.test", "/", false);

        // Act
        let matches = filter.matches(&value);

        // Assert
        assert!(matches);
    }

    #[test]
    fn empty_exact_clear_filter_matches_every_cookie_like_playwright() {
        // Arrange
        let filter = BrowserContextCookieClearFilter {
            name: Some(CookieStringMatcher::exact(String::new()).unwrap()),
            domain: None,
            path: None,
        };
        let value = cookie("example.test", "/", false);

        // Act
        let matches = filter.matches(&value);

        // Assert
        assert!(matches);
    }

    #[test]
    fn filtered_clear_preserves_the_chromium_partition_ancestor_bit() {
        // Arrange
        let cookie = BrowserContextCookie {
            partition_key: Some("https://top-level.test".into()),
            partition_has_cross_site_ancestor: Some(false),
            ..cookie("example.test", "/", true)
        };

        // Act
        let expired = input_to_cdp(expired_cookie(cookie)).unwrap();

        // Assert
        let partition = expired.partition_key.unwrap();
        assert_eq!(partition.top_level_site, "https://top-level.test");
        assert!(!partition.has_cross_site_ancestor);
    }
}
