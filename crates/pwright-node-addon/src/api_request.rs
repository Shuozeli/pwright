//! Thin Node conversion for the optional browser-independent API request owner.

use std::{net::IpAddr, sync::Arc, time::Duration};

#[cfg(feature = "context-api-request")]
use std::collections::HashMap;

#[cfg(feature = "context-api-request")]
use async_trait::async_trait;
use napi::{
    Env,
    bindgen_prelude::{Buffer, PromiseRaw},
};
use napi_derive::napi;
use pwright_api_request::{
    ApiAuthorizationProvider, ApiCookie, ApiCookieStore, ApiHeader, ApiRequest, ApiRequestBody,
    ApiRequestContext, ApiRequestContextOptions, ApiRequestError, ApiRequestLimits, ApiResponse,
    ApiSameSite,
};
use pwright_api_request_auth_basic::{BasicAuthorizationProvider, BasicAuthorizationSend};
#[cfg(feature = "api-request-client-certificates")]
use pwright_api_request_client_certificates::{
    ClientCertificateInput, ClientCertificateMaterial, ClientCertificateStore,
};
#[cfg(feature = "api-request-http-proxy")]
use pwright_api_request_connector_http_proxy::HttpConnectProxyConnector;
#[cfg(feature = "api-request-proxy-bypass")]
use pwright_api_request_connector_proxy_bypass::ProxyBypassConnector;
use pwright_api_request_connector_rustls::RustlsApiConnector;
#[cfg(feature = "api-request-socks5-proxy")]
use pwright_api_request_connector_socks5::Socks5ProxyConnector;
use pwright_api_request_cookie::MemoryCookieStore;
use pwright_api_request_decompress::StandardBodyDecoder;
use pwright_api_request_transport::{ApiConnector, ConnectionApiRequestTransport};
#[cfg(feature = "context-api-request")]
use pwright_node_host::{
    ContextApiRequestOptions, HostError, HostSession,
    context_cookies::{BrowserContextCookie, BrowserContextCookieInput},
};

#[napi(object)]
pub struct NativeApiHeader {
    pub name: String,
    pub value: String,
}

impl From<NativeApiHeader> for ApiHeader {
    fn from(value: NativeApiHeader) -> Self {
        Self::new(value.name, value.value)
    }
}

impl From<&ApiHeader> for NativeApiHeader {
    fn from(value: &ApiHeader) -> Self {
        Self {
            name: value.name.clone(),
            value: value.value.clone(),
        }
    }
}

#[napi(object)]
pub struct NativeApiRequestContextOptions {
    pub base_url: Option<String>,
    pub user_agent: String,
    pub extra_headers: Vec<NativeApiHeader>,
    pub timeout_ms: u32,
    pub fail_on_status_code: bool,
    pub ignore_https_errors: bool,
    pub max_redirects: u32,
    pub http_username: Option<String>,
    pub http_password: Option<String>,
    pub http_origin: Option<String>,
    pub http_send_always: bool,
    pub storage_origins_json: Option<String>,
    pub client_certificates: Vec<NativeApiClientCertificate>,
    pub proxy_server: Option<String>,
    pub proxy_username: Option<String>,
    pub proxy_password: Option<String>,
    pub proxy_bypass: Option<String>,
}

#[napi(object)]
pub struct NativeApiClientCertificate {
    pub origin: String,
    pub cert: Option<Buffer>,
    pub key: Option<Buffer>,
    pub pfx: Option<Buffer>,
    pub passphrase: Option<String>,
}

#[napi(object)]
pub struct NativeApiRequestOptions {
    pub headers: Vec<NativeApiHeader>,
    pub body: Option<Buffer>,
    pub timeout_ms: Option<u32>,
    pub fail_on_status_code: Option<bool>,
    pub ignore_https_errors: Option<bool>,
    pub max_redirects: Option<u32>,
    pub max_retries: u32,
    pub lookup_addresses: Option<Vec<String>>,
}

#[napi(object)]
pub struct NativeApiCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: f64,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: String,
}

impl TryFrom<NativeApiCookie> for ApiCookie {
    type Error = napi::Error;

    fn try_from(value: NativeApiCookie) -> Result<Self, Self::Error> {
        let same_site = match value.same_site.as_str() {
            "Strict" => ApiSameSite::Strict,
            "Lax" => ApiSameSite::Lax,
            "None" => ApiSameSite::None,
            _ => {
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    "cookie sameSite must be Strict, Lax, or None".to_owned(),
                ));
            }
        };
        Ok(Self {
            name: value.name,
            value: value.value,
            domain: value.domain,
            path: value.path,
            expires: value.expires,
            http_only: value.http_only,
            secure: value.secure,
            same_site,
        })
    }
}

impl From<ApiCookie> for NativeApiCookie {
    fn from(value: ApiCookie) -> Self {
        let same_site = match value.same_site {
            ApiSameSite::Strict => "Strict",
            ApiSameSite::Lax => "Lax",
            ApiSameSite::None => "None",
        };
        Self {
            name: value.name,
            value: value.value,
            domain: value.domain,
            path: value.path,
            expires: value.expires,
            http_only: value.http_only,
            secure: value.secure,
            same_site: same_site.to_owned(),
        }
    }
}

#[napi(object)]
pub struct NativeApiRequestMetrics {
    pub disposed: bool,
    pub in_flight: u32,
    pub live_responses: u32,
    pub retained_body_bytes: u32,
    pub retained_log_bytes: u32,
}

#[napi]
pub struct NativeApiRequestContext {
    context: ApiRequestContext,
    storage_origins_json: String,
}

#[cfg(feature = "context-api-request")]
impl NativeApiRequestContext {
    pub(crate) fn for_browser_context(
        host: Arc<HostSession>,
        context_id: String,
        options: ContextApiRequestOptions,
    ) -> napi::Result<Self> {
        let authorization_provider: Option<Arc<dyn ApiAuthorizationProvider>> = options
            .credentials
            .as_ref()
            .map(|credentials| {
                let send = if credentials.send_always {
                    BasicAuthorizationSend::Always
                } else {
                    BasicAuthorizationSend::Unauthorized
                };
                BasicAuthorizationProvider::new(
                    &credentials.username,
                    &credentials.password,
                    credentials.origin.as_deref(),
                    send,
                )
                .map(|provider| Arc::new(provider) as Arc<dyn ApiAuthorizationProvider>)
                .map_err(api_request_display_error)
            })
            .transpose()?;
        let cookie_store: Arc<dyn ApiCookieStore> = Arc::new(BrowserContextCookieStore {
            host,
            context_id: context_id.into_boxed_str(),
        });
        let rustls_connector =
            Arc::new(RustlsApiConnector::new().map_err(api_request_display_error)?);
        #[cfg(feature = "context-proxy")]
        let connector = select_connector(
            rustls_connector,
            options
                .proxy
                .as_ref()
                .map(pwright_node_host::ContextProxy::server),
            options
                .proxy
                .as_ref()
                .and_then(pwright_node_host::ContextProxy::username),
            options
                .proxy
                .as_ref()
                .and_then(pwright_node_host::ContextProxy::password),
            options
                .proxy
                .as_ref()
                .and_then(pwright_node_host::ContextProxy::bypass),
        )?;
        #[cfg(not(feature = "context-proxy"))]
        let connector = select_connector(rustls_connector, None, None, None, None)?;
        let transport = Arc::new(ConnectionApiRequestTransport::with_body_decoder(
            connector,
            Arc::new(StandardBodyDecoder),
        ));
        let context = ApiRequestContext::new(
            transport,
            ApiRequestContextOptions {
                base_url: options.base_url,
                user_agent: options.user_agent,
                extra_headers: Vec::new(),
                timeout: Duration::from_secs(30),
                fail_on_status_code: false,
                ignore_https_errors: options.ignore_https_errors,
                cookie_store: Some(cookie_store),
                authorization_provider,
                max_redirects: 20,
                limits: ApiRequestLimits::default(),
            },
        )
        .map_err(api_request_error)?;
        Ok(Self {
            context,
            storage_origins_json: "[]".into(),
        })
    }
}

#[cfg(feature = "context-api-request")]
struct BrowserContextCookieStore {
    host: Arc<HostSession>,
    context_id: Box<str>,
}

#[cfg(feature = "context-api-request")]
impl std::fmt::Debug for BrowserContextCookieStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BrowserContextCookieStore")
            .field("context_id", &self.context_id)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "context-api-request")]
#[async_trait]
impl ApiCookieStore for BrowserContextCookieStore {
    async fn request_header(
        &self,
        url: &url::Url,
    ) -> Result<Option<String>, pwright_api_request::ApiCookieStoreError> {
        let cookies = self
            .host
            .context_cookies(&self.context_id, vec![url.as_str().to_owned()])
            .await
            .map_err(cookie_host_error)?;
        let values = cookies
            .into_iter()
            .filter(|cookie| cookie.partition_key.is_none())
            .map(|cookie| format!("{}={}", cookie.name, cookie.value))
            .collect::<Vec<_>>();
        Ok((!values.is_empty()).then(|| values.join("; ")))
    }

    async fn store_response_headers(
        &self,
        url: &url::Url,
        values: &[String],
    ) -> Result<(), pwright_api_request::ApiCookieStoreError> {
        if values.is_empty() {
            return Ok(());
        }
        let current = self
            .host
            .context_cookies(&self.context_id, Vec::new())
            .await
            .map_err(cookie_host_error)?
            .into_iter()
            .filter_map(browser_cookie_to_api)
            .collect::<Vec<_>>();
        let memory = MemoryCookieStore::with_defaults()?;
        memory.add_cookies(current.clone()).await?;
        memory.store_response_headers(url, values).await?;
        let next = memory.all_cookies().await?;
        let current_by_key = current
            .into_iter()
            .map(|cookie| (cookie_key(&cookie), cookie))
            .collect::<HashMap<_, _>>();
        let next_by_key = next
            .into_iter()
            .map(|cookie| (cookie_key(&cookie), cookie))
            .collect::<HashMap<_, _>>();
        let mut updates = Vec::new();
        for (key, cookie) in &current_by_key {
            if !next_by_key.contains_key(key) {
                let mut expired = api_cookie_to_browser_input(cookie.clone());
                expired.value.clear();
                expired.expires = Some(0.0);
                updates.push(expired);
            }
        }
        for (key, cookie) in next_by_key {
            if current_by_key.get(&key) != Some(&cookie) {
                updates.push(api_cookie_to_browser_input(cookie));
            }
        }
        if updates.is_empty() {
            return Ok(());
        }
        self.host
            .context_add_cookies(&self.context_id, updates)
            .await
            .map_err(cookie_host_error)
    }

    async fn all_cookies(
        &self,
    ) -> Result<Vec<ApiCookie>, pwright_api_request::ApiCookieStoreError> {
        self.host
            .context_cookies(&self.context_id, Vec::new())
            .await
            .map_err(cookie_host_error)
            .map(|cookies| {
                cookies
                    .into_iter()
                    .filter_map(browser_cookie_to_api)
                    .collect()
            })
    }

    async fn add_cookies(
        &self,
        cookies: Vec<ApiCookie>,
    ) -> Result<(), pwright_api_request::ApiCookieStoreError> {
        self.host
            .context_add_cookies(
                &self.context_id,
                cookies
                    .into_iter()
                    .map(api_cookie_to_browser_input)
                    .collect(),
            )
            .await
            .map_err(cookie_host_error)
    }
}

#[napi]
impl NativeApiRequestContext {
    #[napi(constructor)]
    pub fn new(options: NativeApiRequestContextOptions) -> napi::Result<Self> {
        let storage_origins_json = options
            .storage_origins_json
            .unwrap_or_else(|| "[]".to_owned());
        if storage_origins_json.len() > 8 * 1024 * 1024 {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                "storageState origins exceed the 8 MiB native boundary".to_owned(),
            ));
        }
        let max_redirects = bounded_u16(options.max_redirects, "maxRedirects")?;
        let cookie_store: Arc<dyn ApiCookieStore> =
            Arc::new(MemoryCookieStore::with_defaults().map_err(api_request_display_error)?);
        let authorization_provider: Option<Arc<dyn ApiAuthorizationProvider>> = options
            .http_username
            .as_deref()
            .map(|username| {
                let password = options.http_password.as_deref().unwrap_or_default();
                let send = if options.http_send_always {
                    BasicAuthorizationSend::Always
                } else {
                    BasicAuthorizationSend::Unauthorized
                };
                BasicAuthorizationProvider::new(
                    username,
                    password,
                    options.http_origin.as_deref(),
                    send,
                )
                .map(|provider| Arc::new(provider) as Arc<dyn ApiAuthorizationProvider>)
                .map_err(api_request_display_error)
            })
            .transpose()?;
        #[cfg(feature = "api-request-client-certificates")]
        let rustls_connector: Arc<RustlsApiConnector> = {
            let inputs = options
                .client_certificates
                .into_iter()
                .map(client_certificate_input)
                .collect::<napi::Result<Vec<_>>>()?;
            let provider =
                ClientCertificateStore::new(inputs).map_err(api_request_display_error)?;
            Arc::new(
                RustlsApiConnector::with_client_identity_provider(Arc::new(provider))
                    .map_err(api_request_display_error)?,
            )
        };
        #[cfg(not(feature = "api-request-client-certificates"))]
        let rustls_connector: Arc<RustlsApiConnector> = {
            if !options.client_certificates.is_empty() {
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    "client certificates require the api-request-client-certificates profile"
                        .to_owned(),
                ));
            }
            Arc::new(RustlsApiConnector::new().map_err(api_request_display_error)?)
        };
        let connector = select_connector(
            rustls_connector,
            options.proxy_server.as_deref(),
            options.proxy_username.as_deref(),
            options.proxy_password.as_deref(),
            options.proxy_bypass.as_deref(),
        )?;
        let transport = Arc::new(ConnectionApiRequestTransport::with_body_decoder(
            connector,
            Arc::new(StandardBodyDecoder),
        ));
        let context = ApiRequestContext::new(
            transport,
            ApiRequestContextOptions {
                base_url: options.base_url,
                user_agent: options.user_agent,
                extra_headers: options
                    .extra_headers
                    .into_iter()
                    .map(ApiHeader::from)
                    .collect(),
                timeout: Duration::from_millis(u64::from(options.timeout_ms)),
                fail_on_status_code: options.fail_on_status_code,
                ignore_https_errors: options.ignore_https_errors,
                cookie_store: Some(cookie_store),
                authorization_provider,
                max_redirects,
                limits: ApiRequestLimits::default(),
            },
        )
        .map_err(api_request_error)?;
        Ok(Self {
            context,
            storage_origins_json,
        })
    }

    #[napi]
    pub fn execute<'env>(
        &self,
        env: &'env Env,
        method: String,
        url: String,
        options: NativeApiRequestOptions,
    ) -> napi::Result<PromiseRaw<'env, NativeApiResponse>> {
        let mut request = ApiRequest::new(method, url);
        request.headers = options.headers.into_iter().map(ApiHeader::from).collect();
        if let Some(body) = options.body {
            request.body = ApiRequestBody::Bytes(body.to_vec());
        }
        request.timeout = options
            .timeout_ms
            .map(|value| Duration::from_millis(u64::from(value)));
        request.fail_on_status_code = options.fail_on_status_code;
        request.ignore_https_errors = options.ignore_https_errors;
        request.max_redirects = options
            .max_redirects
            .map(|value| bounded_u16(value, "maxRedirects"))
            .transpose()?;
        request.max_retries = bounded_u16(options.max_retries, "maxRetries")?;
        request.lookup_addresses = options
            .lookup_addresses
            .map(|addresses| {
                addresses
                    .into_iter()
                    .map(|address| {
                        address.parse::<IpAddr>().map_err(|_| {
                            napi::Error::new(
                                napi::Status::InvalidArg,
                                format!("invalid resolver address: {address}"),
                            )
                        })
                    })
                    .collect::<napi::Result<Vec<_>>>()
            })
            .transpose()?;
        let context = self.context.clone();
        env.spawn_future(async move {
            context
                .execute(request)
                .await
                .map(|response| NativeApiResponse { response })
                .map_err(api_request_error)
        })
    }

    #[napi]
    pub fn storage_cookies<'env>(
        &self,
        env: &'env Env,
    ) -> napi::Result<PromiseRaw<'env, Vec<NativeApiCookie>>> {
        let context = self.context.clone();
        env.spawn_future(async move {
            context
                .storage_cookies()
                .await
                .map(|cookies| cookies.into_iter().map(NativeApiCookie::from).collect())
                .map_err(api_request_error)
        })
    }

    #[napi]
    pub fn add_cookies<'env>(
        &self,
        env: &'env Env,
        cookies: Vec<NativeApiCookie>,
    ) -> napi::Result<PromiseRaw<'env, ()>> {
        let cookies = cookies
            .into_iter()
            .map(ApiCookie::try_from)
            .collect::<napi::Result<Vec<_>>>()?;
        let context = self.context.clone();
        env.spawn_future(async move {
            context
                .add_cookies(cookies)
                .await
                .map_err(api_request_error)
        })
    }

    #[napi(getter)]
    pub fn storage_origins_json(&self) -> String {
        self.storage_origins_json.clone()
    }

    #[napi]
    pub fn dispose(&self, reason: Option<String>) {
        self.context.dispose(reason);
    }

    #[napi]
    pub fn metrics(&self) -> napi::Result<NativeApiRequestMetrics> {
        let metrics = self.context.metrics();
        Ok(NativeApiRequestMetrics {
            disposed: metrics.disposed,
            in_flight: bounded_u32(metrics.in_flight, "in-flight request count")?,
            live_responses: bounded_u32(metrics.live_responses, "live response count")?,
            retained_body_bytes: bounded_u32(
                metrics.retained_body_bytes,
                "retained response body bytes",
            )?,
            retained_log_bytes: bounded_u32(
                metrics.retained_log_bytes,
                "retained response log bytes",
            )?,
        })
    }
}

#[cfg(feature = "api-request-client-certificates")]
fn client_certificate_input(
    value: NativeApiClientCertificate,
) -> napi::Result<ClientCertificateInput> {
    let material = match (value.cert, value.key, value.pfx) {
        (Some(certificate), Some(private_key), None) => ClientCertificateMaterial::Pem {
            certificate: certificate.to_vec(),
            private_key: private_key.to_vec(),
            passphrase: value.passphrase,
        },
        (None, None, Some(archive)) => ClientCertificateMaterial::Pfx {
            archive: archive.to_vec(),
            passphrase: value.passphrase,
        },
        _ => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                "client certificate must contain either cert/key or pfx".to_owned(),
            ));
        }
    };
    Ok(ClientCertificateInput {
        origin: value.origin,
        material,
    })
}

impl Drop for NativeApiRequestContext {
    fn drop(&mut self) {
        self.context.dispose(None);
    }
}

#[napi]
pub struct NativeApiResponse {
    response: ApiResponse,
}

#[napi]
impl NativeApiResponse {
    #[napi(getter)]
    pub fn url(&self) -> String {
        self.response.url().to_owned()
    }

    #[napi(getter)]
    pub fn status(&self) -> u32 {
        u32::from(self.response.status())
    }

    #[napi(getter)]
    pub fn status_text(&self) -> String {
        self.response.status_text().to_owned()
    }

    #[napi(getter)]
    pub fn ok(&self) -> bool {
        self.response.ok()
    }

    #[napi]
    pub fn headers_array(&self) -> Vec<NativeApiHeader> {
        self.response
            .headers()
            .iter()
            .map(NativeApiHeader::from)
            .collect()
    }

    #[napi]
    pub fn body(&self) -> napi::Result<Buffer> {
        self.response
            .body()
            .map(Buffer::from)
            .map_err(api_request_error)
    }

    #[napi]
    pub fn log(&self) -> napi::Result<Vec<String>> {
        self.response.log().map_err(api_request_error)
    }

    #[napi]
    pub fn dispose(&self) {
        self.response.dispose();
    }
}

fn select_connector(
    rustls_connector: Arc<RustlsApiConnector>,
    proxy_server: Option<&str>,
    proxy_username: Option<&str>,
    proxy_password: Option<&str>,
    proxy_bypass: Option<&str>,
) -> napi::Result<Arc<dyn ApiConnector>> {
    #[cfg(not(any(
        feature = "api-request-http-proxy",
        feature = "api-request-socks5-proxy"
    )))]
    let _ = (proxy_username, proxy_password);
    let direct_connector: Arc<dyn ApiConnector> = rustls_connector.clone();
    let proxy_connector: Option<Arc<dyn ApiConnector>> = match proxy_server {
        None => None,
        Some(server) if is_socks_proxy(server) => {
            #[cfg(feature = "api-request-socks5-proxy")]
            {
                Some(Arc::new(
                    Socks5ProxyConnector::new(
                        direct_connector.clone(),
                        server,
                        proxy_username,
                        proxy_password,
                    )
                    .map_err(api_request_display_error)?
                    .with_tls_upgrader(rustls_connector.clone()),
                ))
            }
            #[cfg(not(feature = "api-request-socks5-proxy"))]
            {
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    "SOCKS5 proxying requires the api-request-socks5-proxy profile".to_owned(),
                ));
            }
        }
        Some(server) => {
            #[cfg(feature = "api-request-http-proxy")]
            {
                Some(Arc::new(
                    HttpConnectProxyConnector::new(
                        direct_connector.clone(),
                        server,
                        proxy_username,
                        proxy_password,
                    )
                    .map_err(api_request_display_error)?
                    .with_tls_upgrader(rustls_connector.clone()),
                ))
            }
            #[cfg(not(feature = "api-request-http-proxy"))]
            {
                let _ = server;
                return Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    "HTTP proxying requires the api-request-http-proxy profile".to_owned(),
                ));
            }
        }
    };
    match (proxy_connector, proxy_bypass) {
        (None, None) => Ok(direct_connector),
        (None, Some(_)) => Err(napi::Error::new(
            napi::Status::InvalidArg,
            "proxy bypass requires a proxy server".to_owned(),
        )),
        (Some(proxied), None) => Ok(proxied),
        (Some(proxied), Some(bypass)) => {
            #[cfg(feature = "api-request-proxy-bypass")]
            {
                Ok(Arc::new(
                    ProxyBypassConnector::new(direct_connector, proxied, bypass)
                        .map_err(api_request_display_error)?,
                ))
            }
            #[cfg(not(feature = "api-request-proxy-bypass"))]
            {
                let _ = (direct_connector, proxied, bypass);
                Err(napi::Error::new(
                    napi::Status::InvalidArg,
                    "proxy bypass requires the api-request-proxy-bypass profile".to_owned(),
                ))
            }
        }
    }
}

#[cfg(feature = "context-api-request")]
fn cookie_host_error(error: HostError) -> pwright_api_request::ApiCookieStoreError {
    pwright_api_request::ApiCookieStoreError::new(error.to_string())
}

#[cfg(feature = "context-api-request")]
fn cookie_key(cookie: &ApiCookie) -> (String, String, String) {
    (
        cookie.name.clone(),
        cookie.domain.clone(),
        cookie.path.clone(),
    )
}

#[cfg(feature = "context-api-request")]
fn browser_cookie_to_api(cookie: BrowserContextCookie) -> Option<ApiCookie> {
    if cookie.partition_key.is_some() {
        return None;
    }
    let same_site = match cookie.same_site.as_str() {
        "Strict" => ApiSameSite::Strict,
        "None" => ApiSameSite::None,
        _ => ApiSameSite::Lax,
    };
    Some(ApiCookie {
        name: cookie.name,
        value: cookie.value,
        domain: cookie.domain,
        path: cookie.path,
        expires: cookie.expires,
        http_only: cookie.http_only,
        secure: cookie.secure,
        same_site,
    })
}

#[cfg(feature = "context-api-request")]
fn api_cookie_to_browser_input(cookie: ApiCookie) -> BrowserContextCookieInput {
    let same_site = match cookie.same_site {
        ApiSameSite::Strict => "Strict",
        ApiSameSite::Lax => "Lax",
        ApiSameSite::None => "None",
    };
    BrowserContextCookieInput {
        name: cookie.name,
        value: cookie.value,
        url: None,
        domain: Some(cookie.domain),
        path: Some(cookie.path),
        expires: Some(cookie.expires),
        http_only: Some(cookie.http_only),
        secure: Some(cookie.secure),
        same_site: Some(same_site.into()),
        partition_key: None,
        partition_has_cross_site_ancestor: None,
    }
}

fn is_socks_proxy(server: &str) -> bool {
    server
        .split_once("://")
        .and_then(|(scheme, _)| scheme.as_bytes().get(..5))
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"socks"))
}

fn bounded_u16(value: u32, field: &str) -> napi::Result<u16> {
    u16::try_from(value).map_err(|_| {
        napi::Error::new(
            napi::Status::InvalidArg,
            format!("{field} must be between 0 and 65535"),
        )
    })
}

fn bounded_u32(value: usize, field: &str) -> napi::Result<u32> {
    u32::try_from(value).map_err(|_| {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("{field} exceeded the Node boundary"),
        )
    })
}

fn api_request_error(error: ApiRequestError) -> napi::Error {
    api_request_display_error(error)
}

fn api_request_display_error(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::new(napi::Status::GenericFailure, error.to_string())
}
