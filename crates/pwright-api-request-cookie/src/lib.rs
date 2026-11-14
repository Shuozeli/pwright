//! Bounded Playwright-shaped global API request cookie ownership.

use std::{
    fmt,
    sync::{Arc, Mutex, MutexGuard},
    time::{SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use pwright_api_request::{ApiCookie, ApiCookieStore, ApiCookieStoreError, ApiSameSite};
use url::Url;

const MAX_COOKIE_EXPIRES_SECONDS: f64 = 253_402_300_799.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CookieStoreLimits {
    pub max_cookies: usize,
    pub max_cookie_bytes: usize,
    pub max_total_bytes: usize,
}

impl Default for CookieStoreLimits {
    fn default() -> Self {
        Self {
            max_cookies: 1024,
            max_cookie_bytes: 4096,
            max_total_bytes: 1024 * 1024,
        }
    }
}

pub trait CookieClock: Send + Sync + fmt::Debug + 'static {
    fn now_unix_seconds(&self) -> f64;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemCookieClock;

impl CookieClock for SystemCookieClock {
    fn now_unix_seconds(&self) -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0.0, |duration| duration.as_secs_f64())
    }
}

#[derive(Debug, Default)]
struct CookieState {
    buckets: Vec<CookieBucket>,
    cookie_count: usize,
    total_bytes: usize,
    next_ordinal: usize,
}

#[derive(Debug)]
struct CookieBucket {
    name: String,
    cookies: Vec<StoredCookie>,
}

#[derive(Debug)]
struct StoredCookie {
    cookie: ApiCookie,
    ordinal: usize,
}

pub struct MemoryCookieStore {
    limits: CookieStoreLimits,
    clock: Arc<dyn CookieClock>,
    state: Mutex<CookieState>,
}

impl MemoryCookieStore {
    /// Create a bounded cookie store with an injected clock.
    ///
    /// # Errors
    ///
    /// Returns an error when any configured resource limit is zero.
    pub fn new(
        limits: CookieStoreLimits,
        clock: Arc<dyn CookieClock>,
    ) -> Result<Self, ApiCookieStoreError> {
        if limits.max_cookies == 0 || limits.max_cookie_bytes == 0 || limits.max_total_bytes == 0 {
            return Err(ApiCookieStoreError::new(
                "cookie store limits must be nonzero",
            ));
        }
        Ok(Self {
            limits,
            clock,
            state: Mutex::new(CookieState::default()),
        })
    }

    /// Create the standard bounded global API request cookie store.
    ///
    /// # Errors
    ///
    /// Returns an error if the built-in limits are invalid.
    pub fn with_defaults() -> Result<Self, ApiCookieStoreError> {
        Self::new(CookieStoreLimits::default(), Arc::new(SystemCookieClock))
    }

    fn insert(&self, state: &mut CookieState, cookie: ApiCookie) {
        let bucket_index = state
            .buckets
            .iter()
            .position(|bucket| bucket.name == cookie.name)
            .unwrap_or_else(|| {
                state.buckets.push(CookieBucket {
                    name: cookie.name.clone(),
                    cookies: Vec::new(),
                });
                state.buckets.len() - 1
            });
        let replaced = {
            let bucket = &mut state.buckets[bucket_index];
            bucket
                .cookies
                .iter()
                .position(|existing| {
                    existing.cookie.domain == cookie.domain && existing.cookie.path == cookie.path
                })
                .map(|index| bucket.cookies.remove(index))
        };
        if let Some(replaced) = replaced {
            state.cookie_count = state.cookie_count.saturating_sub(1);
            state.total_bytes = state
                .total_bytes
                .saturating_sub(cookie_bytes(&replaced.cookie));
        }
        if expired(&cookie, self.clock.now_unix_seconds()) {
            return;
        }
        let bytes = cookie_bytes(&cookie);
        if bytes > self.limits.max_cookie_bytes || bytes > self.limits.max_total_bytes {
            return;
        }
        let ordinal = allocate_ordinal(state);
        state.buckets[bucket_index]
            .cookies
            .push(StoredCookie { cookie, ordinal });
        state.cookie_count = state.cookie_count.saturating_add(1);
        state.total_bytes = state.total_bytes.saturating_add(bytes);
        while state.cookie_count > self.limits.max_cookies
            || state.total_bytes > self.limits.max_total_bytes
        {
            evict_oldest(state);
        }
    }

    fn prune_expired(&self, state: &mut CookieState) {
        let now = self.clock.now_unix_seconds();
        let mut removed_count = 0_usize;
        let mut removed_bytes = 0_usize;
        for bucket in &mut state.buckets {
            bucket.cookies.retain(|stored| {
                if expired(&stored.cookie, now) {
                    removed_count = removed_count.saturating_add(1);
                    removed_bytes = removed_bytes.saturating_add(cookie_bytes(&stored.cookie));
                    false
                } else {
                    true
                }
            });
        }
        state.cookie_count = state.cookie_count.saturating_sub(removed_count);
        state.total_bytes = state.total_bytes.saturating_sub(removed_bytes);
        state.buckets.retain(|bucket| !bucket.cookies.is_empty());
    }
}

impl fmt::Debug for MemoryCookieStore {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MemoryCookieStore")
            .field("limits", &self.limits)
            .field("cookies", &lock(&self.state).cookie_count)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl ApiCookieStore for MemoryCookieStore {
    async fn request_header(&self, url: &Url) -> Result<Option<String>, ApiCookieStoreError> {
        let mut state = lock(&self.state);
        self.prune_expired(&mut state);
        let values = state
            .buckets
            .iter()
            .flat_map(|bucket| &bucket.cookies)
            .filter(|stored| cookie_matches(&stored.cookie, url))
            .map(|stored| format!("{}={}", stored.cookie.name, stored.cookie.value))
            .collect::<Vec<_>>();
        Ok((!values.is_empty()).then(|| values.join("; ")))
    }

    async fn store_response_headers(
        &self,
        url: &Url,
        values: &[String],
    ) -> Result<(), ApiCookieStoreError> {
        let mut state = lock(&self.state);
        for value in values {
            if value.len() > self.limits.max_cookie_bytes {
                continue;
            }
            if let Some(cookie) = parse_set_cookie(value, url, self.clock.now_unix_seconds()) {
                self.insert(&mut state, cookie);
            }
        }
        self.prune_expired(&mut state);
        Ok(())
    }

    async fn all_cookies(&self) -> Result<Vec<ApiCookie>, ApiCookieStoreError> {
        let mut state = lock(&self.state);
        self.prune_expired(&mut state);
        Ok(state
            .buckets
            .iter()
            .flat_map(|bucket| &bucket.cookies)
            .map(|stored| stored.cookie.clone())
            .collect())
    }

    async fn add_cookies(&self, cookies: Vec<ApiCookie>) -> Result<(), ApiCookieStoreError> {
        let mut state = lock(&self.state);
        for mut cookie in cookies {
            validate_structured_cookie(&cookie)?;
            cookie.domain.make_ascii_lowercase();
            self.insert(&mut state, cookie);
        }
        self.prune_expired(&mut state);
        Ok(())
    }
}

fn allocate_ordinal(state: &mut CookieState) -> usize {
    if state.next_ordinal != usize::MAX {
        let ordinal = state.next_ordinal;
        state.next_ordinal += 1;
        return ordinal;
    }
    let mut positions = state
        .buckets
        .iter()
        .enumerate()
        .flat_map(|(bucket_index, bucket)| {
            bucket
                .cookies
                .iter()
                .enumerate()
                .map(move |(cookie_index, cookie)| (cookie.ordinal, bucket_index, cookie_index))
        })
        .collect::<Vec<_>>();
    positions.sort_unstable_by_key(|position| position.0);
    for (ordinal, (_, bucket_index, cookie_index)) in positions.into_iter().enumerate() {
        state.buckets[bucket_index].cookies[cookie_index].ordinal = ordinal;
    }
    let ordinal = state.cookie_count;
    state.next_ordinal = ordinal.saturating_add(1);
    ordinal
}

fn evict_oldest(state: &mut CookieState) {
    let oldest = state
        .buckets
        .iter()
        .enumerate()
        .flat_map(|(bucket_index, bucket)| {
            bucket
                .cookies
                .iter()
                .enumerate()
                .map(move |(cookie_index, cookie)| (cookie.ordinal, bucket_index, cookie_index))
        })
        .min_by_key(|position| position.0);
    let Some((_, bucket_index, cookie_index)) = oldest else {
        return;
    };
    let removed = state.buckets[bucket_index].cookies.remove(cookie_index);
    state.cookie_count = state.cookie_count.saturating_sub(1);
    state.total_bytes = state
        .total_bytes
        .saturating_sub(cookie_bytes(&removed.cookie));
    if state.buckets[bucket_index].cookies.is_empty() {
        state.buckets.remove(bucket_index);
    }
}

fn parse_set_cookie(header: &str, url: &Url, now: f64) -> Option<ApiCookie> {
    let pairs = header
        .split(';')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }
            let (name, value) = part.split_once('=').unwrap_or((part, ""));
            Some((name.trim(), value.trim()))
        })
        .collect::<Vec<_>>();
    let (name, value) = *pairs.first()?;
    let mut cookie = ApiCookie {
        name: name.into(),
        value: value.into(),
        domain: String::new(),
        path: String::new(),
        expires: -1.0,
        http_only: false,
        secure: false,
        same_site: ApiSameSite::Lax,
    };
    for (attribute, value) in pairs.iter().skip(1) {
        match attribute.to_ascii_lowercase().as_str() {
            "expires" => {
                if let Some(expires) = parse_cookie_date(value) {
                    cookie.expires = expires;
                }
            }
            "max-age" => {
                if let Some(max_age) = parse_number_prefix(value) {
                    cookie.expires = if max_age <= 0.0 {
                        0.0
                    } else {
                        (now + max_age).min(MAX_COOKIE_EXPIRES_SECONDS)
                    };
                }
            }
            "domain" => {
                cookie.domain = value.to_ascii_lowercase();
                if !cookie.domain.is_empty()
                    && !cookie.domain.starts_with('.')
                    && cookie.domain.contains('.')
                {
                    cookie.domain.insert(0, '.');
                }
            }
            "path" => cookie.path = (*value).into(),
            "secure" => cookie.secure = true,
            "httponly" => cookie.http_only = true,
            "samesite" if value.eq_ignore_ascii_case("none") => {
                cookie.same_site = ApiSameSite::None;
            }
            "samesite" if value.eq_ignore_ascii_case("strict") => {
                cookie.same_site = ApiSameSite::Strict;
            }
            "samesite" if value.eq_ignore_ascii_case("lax") => {
                cookie.same_site = ApiSameSite::Lax;
            }
            _ => {}
        }
    }

    let hostname = url.host_str()?.to_ascii_lowercase();
    if cookie.domain.is_empty() {
        cookie.domain = hostname;
    } else if !domain_matches(&hostname, &cookie.domain) {
        return None;
    }
    if !cookie.path.starts_with('/') {
        cookie.path = default_cookie_path(url.path());
    }
    Some(cookie)
}

fn parse_cookie_date(value: &str) -> Option<f64> {
    if let Ok(time) = httpdate::parse_http_date(value) {
        return Some(
            time.duration_since(UNIX_EPOCH)
                .map_or(0.0, |duration| duration.as_secs_f64())
                .min(MAX_COOKIE_EXPIRES_SECONDS),
        );
    }
    let contains_pre_epoch_year = value
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| part.len() == 4)
        .filter_map(|part| part.parse::<u16>().ok())
        .any(|year| year < 1970);
    contains_pre_epoch_year.then_some(0.0)
}

fn parse_number_prefix(value: &str) -> Option<f64> {
    let value = value.trim();
    let sign_bytes = usize::from(value.starts_with(['-', '+']));
    let digit_bytes = value[sign_bytes..]
        .bytes()
        .take_while(u8::is_ascii_digit)
        .count();
    if digit_bytes == 0 {
        return None;
    }
    value[..sign_bytes + digit_bytes]
        .parse()
        .ok()
        .filter(|number: &f64| number.is_finite())
}

fn default_cookie_path(path: &str) -> String {
    let mut segments = path.split('/').collect::<Vec<_>>();
    segments.pop();
    let joined = segments.join("/");
    if joined.is_empty() {
        "/".into()
    } else {
        format!("/{joined}").replace("//", "/")
    }
}

fn cookie_matches(cookie: &ApiCookie, url: &Url) -> bool {
    let Some(hostname) = url.host_str() else {
        return false;
    };
    if cookie.secure && url.scheme() != "https" && !is_local_hostname(hostname) {
        return false;
    }
    domain_matches(&hostname.to_ascii_lowercase(), &cookie.domain)
        && path_matches(url.path(), &cookie.path)
}

fn domain_matches(hostname: &str, domain: &str) -> bool {
    if hostname.eq_ignore_ascii_case(domain) {
        return true;
    }
    if !domain.starts_with('.') {
        return false;
    }
    format!(".{hostname}")
        .to_ascii_lowercase()
        .ends_with(&domain.to_ascii_lowercase())
}

fn path_matches(request_path: &str, cookie_path: &str) -> bool {
    if request_path == cookie_path {
        return true;
    }
    let request_path = if request_path.ends_with('/') {
        request_path.into()
    } else {
        format!("{request_path}/")
    };
    let cookie_path = if cookie_path.ends_with('/') {
        cookie_path.into()
    } else {
        format!("{cookie_path}/")
    };
    request_path.starts_with(&cookie_path)
}

fn is_local_hostname(hostname: &str) -> bool {
    hostname.eq_ignore_ascii_case("localhost")
        || hostname.to_ascii_lowercase().ends_with(".localhost")
}

fn expired(cookie: &ApiCookie, now: f64) -> bool {
    cookie.expires >= 0.0 && cookie.expires < now
}

fn validate_structured_cookie(cookie: &ApiCookie) -> Result<(), ApiCookieStoreError> {
    if cookie.domain.is_empty() || cookie.path.is_empty() || !cookie.path.starts_with('/') {
        return Err(ApiCookieStoreError::new(
            "cookie domain and absolute path are required",
        ));
    }
    if !cookie.expires.is_finite()
        || (cookie.expires < 0.0 && cookie.expires.to_bits() != (-1.0_f64).to_bits())
        || cookie.expires > MAX_COOKIE_EXPIRES_SECONDS
    {
        return Err(ApiCookieStoreError::new("cookie expiry is invalid"));
    }
    Ok(())
}

fn cookie_bytes(cookie: &ApiCookie) -> usize {
    cookie
        .name
        .len()
        .saturating_add(cookie.value.len())
        .saturating_add(cookie.domain.len())
        .saturating_add(cookie.path.len())
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_api_request::{ApiCookie, ApiCookieStore, ApiSameSite};
    use url::Url;

    use super::{CookieClock, CookieStoreLimits, MemoryCookieStore};

    #[derive(Debug)]
    struct FixedClock(f64);

    impl CookieClock for FixedClock {
        fn now_unix_seconds(&self) -> f64 {
            self.0
        }
    }

    fn test_store() -> MemoryCookieStore {
        MemoryCookieStore::new(CookieStoreLimits::default(), Arc::new(FixedClock(1000.0)))
            .expect("test store should build")
    }

    #[tokio::test]
    async fn response_cookie_defaults_and_equal_signs_are_preserved() {
        // Arrange
        let store = test_store();
        let url =
            Url::parse("http://a.b.one.com/dir/setcookie.html").expect("test URL should parse");

        // Act
        store
            .store_response_headers(
                &url,
                &["f=value == value=; secure; httpOnly; path=/some=value".into()],
            )
            .await
            .expect("cookie should store");
        let cookies = store.all_cookies().await.expect("cookies should export");

        // Assert
        assert_eq!(cookies.len(), 1);
        assert_eq!(cookies[0].name, "f");
        assert_eq!(cookies[0].value, "value == value=");
        assert_eq!(cookies[0].domain, "a.b.one.com");
        assert_eq!(cookies[0].path, "/some=value");
        assert!(cookies[0].secure);
        assert!(cookies[0].http_only);
        assert_eq!(cookies[0].same_site, ApiSameSite::Lax);
    }

    #[tokio::test]
    async fn request_header_filters_domain_path_and_preserves_order() {
        // Arrange
        let store = test_store();
        let source =
            Url::parse("http://a.b.one.com/setcookie.html").expect("source URL should parse");
        store
            .store_response_headers(
                &source,
                &[
                    "a=v; path=/input/subfolder".into(),
                    "b=v; path=/input".into(),
                    "c=v".into(),
                    "d=v; domain=other.com".into(),
                ],
            )
            .await
            .expect("cookies should store");
        let target =
            Url::parse("http://a.b.one.com/input/button.html").expect("target URL should parse");

        // Act
        let header = store
            .request_header(&target)
            .await
            .expect("header should resolve");

        // Assert
        assert_eq!(header.as_deref(), Some("b=v; c=v"));
    }

    #[tokio::test]
    async fn secure_cookie_allows_https_and_localhost_http_only() {
        // Arrange
        let store = test_store();
        let source = Url::parse("http://example.test/").expect("source URL should parse");
        store
            .store_response_headers(&source, &["a=v; secure".into()])
            .await
            .expect("cookie should store");
        let localhost_store = test_store();
        let localhost = Url::parse("http://a.localhost/").expect("localhost URL should parse");
        localhost_store
            .store_response_headers(&localhost, &["a=v; secure".into()])
            .await
            .expect("localhost cookie should store");

        // Act
        let remote_http = store
            .request_header(&source)
            .await
            .expect("remote header should resolve");
        let remote_https = store
            .request_header(&Url::parse("https://example.test/").expect("HTTPS URL should parse"))
            .await
            .expect("HTTPS header should resolve");
        let local_http = localhost_store
            .request_header(&localhost)
            .await
            .expect("localhost header should resolve");

        // Assert
        assert_eq!(remote_http, None);
        assert_eq!(remote_https.as_deref(), Some("a=v"));
        assert_eq!(local_http.as_deref(), Some("a=v"));
    }

    #[tokio::test]
    async fn expired_and_negative_max_age_cookies_remove_existing_identity() {
        // Arrange
        let store = test_store();
        let url = Url::parse("http://example.test/").expect("test URL should parse");
        store
            .store_response_headers(&url, &["a=old; max-age=100".into(), "b=ok".into()])
            .await
            .expect("initial cookies should store");

        // Act
        store
            .store_response_headers(&url, &["a=gone; max-age=-1".into()])
            .await
            .expect("expiry cookie should store");
        let header = store
            .request_header(&url)
            .await
            .expect("header should resolve");

        // Assert
        assert_eq!(header.as_deref(), Some("b=ok"));
    }

    #[tokio::test]
    async fn pre_epoch_expires_removes_existing_cookie_identity() {
        // Arrange
        let store = test_store();
        let url = Url::parse("http://example.test/").expect("test URL should parse");
        store
            .store_response_headers(&url, &["a=old".into(), "b=ok".into()])
            .await
            .expect("initial cookies should store");

        // Act
        store
            .store_response_headers(
                &url,
                &["a=gone; expires=1 Jan 1000 00:00:00 +0000 (UTC)".into()],
            )
            .await
            .expect("expiry cookie should store");
        let header = store
            .request_header(&url)
            .await
            .expect("header should resolve");

        // Assert
        assert_eq!(header.as_deref(), Some("b=ok"));
    }

    #[tokio::test]
    async fn domain_attribute_is_case_insensitive_and_matches_subdomains() {
        // Arrange
        let store = test_store();
        let source = Url::parse("http://a.b.one.com/").expect("source URL should parse");
        store
            .store_response_headers(&source, &["a=v; domain=ONE.COM".into()])
            .await
            .expect("domain cookie should store");
        let sibling = Url::parse("http://c.one.com/").expect("sibling URL should parse");
        let unrelated = Url::parse("http://notone.com/").expect("unrelated URL should parse");

        // Act
        let sibling_header = store
            .request_header(&sibling)
            .await
            .expect("sibling header should resolve");
        let unrelated_header = store
            .request_header(&unrelated)
            .await
            .expect("unrelated header should resolve");
        let cookies = store.all_cookies().await.expect("cookies should export");

        // Assert
        assert_eq!(sibling_header.as_deref(), Some("a=v"));
        assert_eq!(unrelated_header, None);
        assert_eq!(cookies[0].domain, ".one.com");
    }

    #[tokio::test]
    async fn structured_cookie_override_preserves_name_bucket_order() {
        // Arrange
        let store = test_store();
        let cookie = |name: &str, value: &str| ApiCookie {
            name: name.into(),
            value: value.into(),
            domain: "example.test".into(),
            path: "/".into(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: ApiSameSite::Lax,
        };
        store
            .add_cookies(vec![cookie("a", "old"), cookie("b", "same")])
            .await
            .expect("initial cookies should store");

        // Act
        store
            .add_cookies(vec![cookie("a", "new")])
            .await
            .expect("replacement should store");
        let cookies = store.all_cookies().await.expect("cookies should export");

        // Assert
        assert_eq!(cookies[0].name, "a");
        assert_eq!(cookies[0].value, "new");
        assert_eq!(cookies[1].name, "b");
    }

    #[tokio::test]
    async fn cookies_are_grouped_by_first_name_then_identity_insertion() {
        // Arrange
        let store = test_store();
        let cookie = |name: &str, value: &str, path: &str| ApiCookie {
            name: name.into(),
            value: value.into(),
            domain: "example.test".into(),
            path: path.into(),
            expires: -1.0,
            http_only: false,
            secure: false,
            same_site: ApiSameSite::Lax,
        };
        store
            .add_cookies(vec![
                cookie("a", "root", "/"),
                cookie("b", "value", "/"),
                cookie("a", "nested", "/nested"),
            ])
            .await
            .expect("cookies should store");
        let target = Url::parse("http://example.test/nested/page").expect("URL should parse");

        // Act
        let header = store
            .request_header(&target)
            .await
            .expect("header should resolve");

        // Assert
        assert_eq!(header.as_deref(), Some("a=root; a=nested; b=value"));
    }

    #[tokio::test]
    async fn count_limit_evicts_oldest_cookie_without_unbounded_growth() {
        // Arrange
        let limits = CookieStoreLimits {
            max_cookies: 2,
            max_cookie_bytes: 64,
            max_total_bytes: 128,
        };
        let store = MemoryCookieStore::new(limits, Arc::new(FixedClock(1000.0)))
            .expect("test store should build");
        let url = Url::parse("http://example.test/").expect("test URL should parse");

        // Act
        store
            .store_response_headers(&url, &["a=1".into(), "b=2".into(), "c=3".into()])
            .await
            .expect("cookies should store");
        let cookies = store.all_cookies().await.expect("cookies should export");

        // Assert
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies[0].name, "b");
        assert_eq!(cookies[1].name, "c");
    }
}
