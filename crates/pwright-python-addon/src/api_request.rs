//! Thin Python conversion for the optional browser-independent API request owner.

use std::{collections::BTreeMap, net::IpAddr, sync::Arc, time::Duration};

#[cfg(feature = "context-api-request")]
use std::collections::HashMap;

#[cfg(feature = "context-api-request")]
use async_trait::async_trait;
use pwright_api_request::{
    ApiAuthorizationProvider, ApiCookie, ApiCookieStore, ApiFilePayload, ApiHeader,
    ApiMultipartField, ApiNameValue, ApiQuery, ApiRequest, ApiRequestBody, ApiRequestContext,
    ApiRequestContextOptions, ApiRequestError, ApiRequestLimits, ApiResponse, ApiSameSite,
};
use pwright_api_request_auth_basic::{BasicAuthorizationProvider, BasicAuthorizationSend};
#[cfg(feature = "api-request-client-certificates")]
use pwright_api_request_client_certificates::{
    ClientCertificateInput, ClientCertificateMaterial, ClientCertificateStore,
};
use pwright_api_request_connector_rustls::RustlsApiConnector;
use pwright_api_request_cookie::MemoryCookieStore;
use pwright_api_request_decompress::StandardBodyDecoder;
use pwright_api_request_transport::{ApiConnector, ConnectionApiRequestTransport};
use pyo3::{prelude::*, types::PyBytes};

#[cfg(any(
    feature = "route-fulfill-api-response",
    feature = "context-api-request"
))]
use pwright_node_host::HostError;
#[cfg(feature = "context-api-request")]
use pwright_node_host::{
    ContextApiRequestOptions, HostSession,
    context_cookies::{BrowserContextCookie, BrowserContextCookieInput},
};
#[cfg(feature = "route-fulfill-api-response")]
use pwright_node_host::{MAX_ROUTE_FULFILL_BODY_BYTES, RouteFulfill};

use crate::Error;

type PythonHeader = (String, String);
type PythonCookie = (String, String, String, String, f64, bool, bool, String);
type PythonContextOptions = (
    Option<String>,
    String,
    Vec<PythonHeader>,
    u32,
    bool,
    bool,
    u32,
);
type PythonCredentials = (String, String, Option<String>, bool);
type PythonClientCertificate = (
    String,
    Option<Py<PyBytes>>,
    Option<Py<PyBytes>>,
    Option<Py<PyBytes>>,
    Option<String>,
);
type PythonRequestControls = (
    Option<u32>,
    Option<bool>,
    Option<bool>,
    Option<u32>,
    u32,
    Option<Vec<String>>,
);
type PythonQuery = (Option<String>, Vec<(String, String)>);
type PythonMultipartField = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<Py<PyBytes>>,
);
type PythonBody = (
    u8,
    Option<Py<PyBytes>>,
    Option<String>,
    Vec<(String, String)>,
    Vec<PythonMultipartField>,
);

fn create_api_request_context(
    options: PythonContextOptions,
    credentials: Option<PythonCredentials>,
    cookie_store: Arc<dyn ApiCookieStore>,
    connector: Arc<dyn ApiConnector>,
) -> PyResult<ApiRequestContext> {
    let (
        base_url,
        user_agent,
        extra_headers,
        timeout_ms,
        fail_on_status_code,
        ignore_https_errors,
        max_redirects,
    ) = options;
    let max_redirects = bounded_u16(max_redirects, "max_redirects")?;
    let authorization_provider: Option<Arc<dyn ApiAuthorizationProvider>> = credentials
        .as_ref()
        .map(|(username, password, origin, send_always)| {
            let send = if *send_always {
                BasicAuthorizationSend::Always
            } else {
                BasicAuthorizationSend::Unauthorized
            };
            BasicAuthorizationProvider::new(username, password, origin.as_deref(), send)
                .map(|provider| Arc::new(provider) as Arc<dyn ApiAuthorizationProvider>)
                .map_err(display_error)
        })
        .transpose()?;
    let transport = Arc::new(ConnectionApiRequestTransport::with_body_decoder(
        connector,
        Arc::new(StandardBodyDecoder),
    ));
    ApiRequestContext::new(
        transport,
        ApiRequestContextOptions {
            base_url,
            user_agent,
            extra_headers: extra_headers
                .into_iter()
                .map(|(name, value)| ApiHeader::new(name, value))
                .collect(),
            timeout: Duration::from_millis(u64::from(timeout_ms)),
            fail_on_status_code,
            ignore_https_errors,
            cookie_store: Some(cookie_store),
            authorization_provider,
            max_redirects,
            limits: ApiRequestLimits::default(),
        },
    )
    .map_err(api_request_error)
}

#[pyclass(module = "pwright._native")]
pub(crate) struct NativeApiRequestContext {
    context: ApiRequestContext,
    storage_origins_json: String,
}

#[cfg(feature = "context-api-request")]
impl NativeApiRequestContext {
    pub(crate) fn for_browser_context(
        host: Arc<HostSession>,
        context_id: String,
        options: ContextApiRequestOptions,
    ) -> PyResult<Self> {
        let credentials = options.credentials.map(|credentials| {
            (
                credentials.username,
                credentials.password,
                credentials.origin,
                credentials.send_always,
            )
        });
        let cookie_store: Arc<dyn ApiCookieStore> = Arc::new(BrowserContextCookieStore {
            host,
            context_id: context_id.into_boxed_str(),
        });
        let connector: Arc<dyn ApiConnector> =
            Arc::new(RustlsApiConnector::new().map_err(display_error)?);
        let context = create_api_request_context(
            (
                options.base_url,
                options.user_agent,
                Vec::new(),
                30_000,
                false,
                options.ignore_https_errors,
                20,
            ),
            credentials,
            cookie_store,
            connector,
        )?;
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

#[pymethods]
impl NativeApiRequestContext {
    #[new]
    fn new(
        py: Python<'_>,
        options: PythonContextOptions,
        credentials: Option<PythonCredentials>,
        client_certificates: Vec<PythonClientCertificate>,
        storage_origins_json: String,
    ) -> PyResult<Self> {
        let _ = py;
        if storage_origins_json.len() > 8 * 1024 * 1024 {
            return Err(Error::new_err(
                "storageState origins exceed the 8 MiB native boundary",
            ));
        }
        let cookie_store: Arc<dyn ApiCookieStore> =
            Arc::new(MemoryCookieStore::with_defaults().map_err(display_error)?);
        #[cfg(feature = "api-request-client-certificates")]
        let connector: Arc<dyn ApiConnector> = {
            let inputs = client_certificates
                .into_iter()
                .map(|certificate| client_certificate_input(py, certificate))
                .collect::<PyResult<Vec<_>>>()?;
            let provider = ClientCertificateStore::new(inputs).map_err(display_error)?;
            Arc::new(
                RustlsApiConnector::with_client_identity_provider(Arc::new(provider))
                    .map_err(display_error)?,
            )
        };
        #[cfg(not(feature = "api-request-client-certificates"))]
        let connector: Arc<dyn ApiConnector> = {
            if !client_certificates.is_empty() {
                return Err(Error::new_err(
                    "client certificates require the api-request-client-certificates profile",
                ));
            }
            Arc::new(RustlsApiConnector::new().map_err(display_error)?)
        };
        let context = create_api_request_context(options, credentials, cookie_store, connector)?;
        Ok(Self {
            context,
            storage_origins_json,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "the PyO3 ABI keeps method, URL, headers, query, body, and controls independently typed"
    )]
    fn execute<'py>(
        &self,
        py: Python<'py>,
        method: String,
        url: String,
        headers: Vec<PythonHeader>,
        query: PythonQuery,
        body: PythonBody,
        controls: PythonRequestControls,
    ) -> PyResult<Bound<'py, PyAny>> {
        let (
            timeout_ms,
            fail_on_status_code,
            ignore_https_errors,
            max_redirects,
            max_retries,
            lookup_addresses,
        ) = controls;
        let mut request = ApiRequest::new(method, url);
        request.headers = headers
            .into_iter()
            .map(|(name, value)| ApiHeader::new(name, value))
            .collect();
        request.query = python_query(query)?;
        request.body = python_body(py, body)?;
        request.timeout = timeout_ms.map(|value| Duration::from_millis(u64::from(value)));
        request.fail_on_status_code = fail_on_status_code;
        request.ignore_https_errors = ignore_https_errors;
        request.max_redirects = max_redirects
            .map(|value| bounded_u16(value, "max_redirects"))
            .transpose()?;
        request.max_retries = bounded_u16(max_retries, "max_retries")?;
        request.lookup_addresses = lookup_addresses
            .map(|addresses| {
                addresses
                    .into_iter()
                    .map(|address| {
                        address.parse::<IpAddr>().map_err(|_| {
                            Error::new_err(format!("invalid resolver address: {address}"))
                        })
                    })
                    .collect::<PyResult<Vec<_>>>()
            })
            .transpose()?;
        let context = self.context.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            let response = context.execute(request).await.map_err(api_request_error)?;
            Python::attach(|py| Py::new(py, NativeApiResponse { response }))
        })
    }

    fn storage_cookies<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let context = self.context.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            context
                .storage_cookies()
                .await
                .map(|cookies| {
                    cookies
                        .into_iter()
                        .map(cookie_to_python)
                        .collect::<Vec<_>>()
                })
                .map_err(api_request_error)
        })
    }

    fn add_cookies<'py>(
        &self,
        py: Python<'py>,
        cookies: Vec<PythonCookie>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let cookies = cookies
            .into_iter()
            .map(cookie_from_python)
            .collect::<PyResult<Vec<_>>>()?;
        let context = self.context.clone();
        pyo3_async_runtimes::tokio::future_into_py(py, async move {
            context
                .add_cookies(cookies)
                .await
                .map_err(api_request_error)
        })
    }

    #[getter]
    fn storage_origins_json(&self) -> String {
        self.storage_origins_json.clone()
    }

    fn dispose(&self, reason: Option<String>) {
        self.context.dispose(reason);
    }

    fn metrics(&self) -> BTreeMap<&'static str, u64> {
        let metrics = self.context.metrics();
        BTreeMap::from([
            ("disposed", u64::from(metrics.disposed)),
            ("in_flight", metrics.in_flight as u64),
            ("live_responses", metrics.live_responses as u64),
            ("retained_body_bytes", metrics.retained_body_bytes as u64),
            ("retained_log_bytes", metrics.retained_log_bytes as u64),
        ])
    }
}

fn python_query(query: PythonQuery) -> PyResult<ApiQuery> {
    let (encoded, pairs) = query;
    match (encoded, pairs.is_empty()) {
        (Some(encoded), true) => Ok(ApiQuery::Encoded(encoded)),
        (None, false) => Ok(ApiQuery::Pairs(
            pairs
                .into_iter()
                .map(|(name, value)| ApiNameValue::new(name, value))
                .collect(),
        )),
        (None, true) => Ok(ApiQuery::None),
        (Some(_), false) => Err(Error::new_err(
            "request query cannot contain encoded and structured parameters",
        )),
    }
}

fn python_body(py: Python<'_>, body: PythonBody) -> PyResult<ApiRequestBody> {
    let (kind, bytes, json, fields, multipart) = body;
    match kind {
        0 if bytes.is_none() && json.is_none() && fields.is_empty() && multipart.is_empty() => {
            Ok(ApiRequestBody::None)
        }
        1 if json.is_none() && fields.is_empty() && multipart.is_empty() => bytes
            .map(|value| ApiRequestBody::Bytes(value.bind(py).as_bytes().to_vec()))
            .ok_or_else(|| Error::new_err("byte request body is missing its payload")),
        2 if bytes.is_none() && fields.is_empty() && multipart.is_empty() => json
            .map(ApiRequestBody::Json)
            .ok_or_else(|| Error::new_err("JSON request body is missing its payload")),
        3 if bytes.is_none() && json.is_none() && multipart.is_empty() => Ok(ApiRequestBody::Form(
            fields
                .into_iter()
                .map(|(name, value)| ApiNameValue::new(name, value))
                .collect(),
        )),
        4 if bytes.is_none() && json.is_none() && fields.is_empty() => {
            Ok(ApiRequestBody::Multipart(
                multipart
                    .into_iter()
                    .map(|(name, value, file_name, mime_type, buffer)| {
                        match (value, file_name, buffer) {
                            (Some(value), None, None) => Ok(ApiMultipartField::text(name, value)),
                            (None, Some(file_name), Some(buffer)) => {
                                let mut file = ApiFilePayload::new(
                                    file_name,
                                    buffer.bind(py).as_bytes().to_vec(),
                                );
                                file.mime_type = mime_type;
                                Ok(ApiMultipartField::file(name, file))
                            }
                            _ => Err(Error::new_err(
                                "multipart field must contain exactly one text or file payload",
                            )),
                        }
                    })
                    .collect::<PyResult<Vec<_>>>()?,
            ))
        }
        _ => Err(Error::new_err("invalid native request body descriptor")),
    }
}

#[cfg(feature = "api-request-client-certificates")]
fn client_certificate_input(
    py: Python<'_>,
    value: PythonClientCertificate,
) -> PyResult<ClientCertificateInput> {
    let (origin, certificate, private_key, archive, passphrase) = value;
    let material = match (certificate, private_key, archive) {
        (Some(certificate), Some(private_key), None) => ClientCertificateMaterial::Pem {
            certificate: certificate.bind(py).as_bytes().to_vec(),
            private_key: private_key.bind(py).as_bytes().to_vec(),
            passphrase,
        },
        (None, None, Some(archive)) => ClientCertificateMaterial::Pfx {
            archive: archive.bind(py).as_bytes().to_vec(),
            passphrase,
        },
        _ => {
            return Err(Error::new_err(
                "client certificate must contain either cert/key or pfx",
            ));
        }
    };
    Ok(ClientCertificateInput { origin, material })
}

impl Drop for NativeApiRequestContext {
    fn drop(&mut self) {
        self.context.dispose(None);
    }
}

#[pyclass(module = "pwright._native")]
pub(crate) struct NativeApiResponse {
    response: ApiResponse,
}

#[cfg(feature = "route-fulfill-api-response")]
impl NativeApiResponse {
    pub(crate) fn route_fulfill(
        &self,
        status: Option<i64>,
        headers: Option<Vec<PythonHeader>>,
        content_type: Option<String>,
    ) -> Result<RouteFulfill, HostError> {
        let status = status.unwrap_or_else(|| i64::from(self.response.status()));
        let mut headers = headers.unwrap_or_else(|| {
            self.response
                .headers()
                .iter()
                .map(|header| (header.name.clone(), header.value.clone()))
                .collect()
        });
        if let Some(content_type) = content_type {
            headers.retain(|(name, _)| !name.eq_ignore_ascii_case("content-type"));
            headers.push(("content-type".into(), content_type));
        }
        let body = self
            .response
            .body_bounded(MAX_ROUTE_FULFILL_BODY_BYTES)
            .map_err(|error| match error {
                ApiRequestError::Capacity(_) => HostError::ResourceExhausted(error.to_string()),
                _ => HostError::Backend(error.to_string()),
            })?;
        RouteFulfill::new(status, headers, body)
    }
}

#[pymethods]
impl NativeApiResponse {
    #[getter]
    fn url(&self) -> String {
        self.response.url().to_owned()
    }

    #[getter]
    fn status(&self) -> u32 {
        u32::from(self.response.status())
    }

    #[getter]
    fn status_text(&self) -> String {
        self.response.status_text().to_owned()
    }

    #[getter]
    fn ok(&self) -> bool {
        self.response.ok()
    }

    fn headers_array(&self) -> Vec<PythonHeader> {
        self.response
            .headers()
            .iter()
            .map(|header| (header.name.clone(), header.value.clone()))
            .collect()
    }

    fn body(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        let body = self.response.body().map_err(api_request_error)?;
        Ok(PyBytes::new(py, &body).unbind())
    }

    fn log(&self) -> PyResult<Vec<String>> {
        self.response.log().map_err(api_request_error)
    }

    fn dispose(&self) {
        self.response.dispose();
    }
}

fn cookie_from_python(cookie: PythonCookie) -> PyResult<ApiCookie> {
    let (name, value, domain, path, expires, http_only, secure, same_site) = cookie;
    let same_site = match same_site.as_str() {
        "Strict" => ApiSameSite::Strict,
        "Lax" => ApiSameSite::Lax,
        "None" => ApiSameSite::None,
        _ => {
            return Err(Error::new_err(
                "cookie sameSite must be Strict, Lax, or None",
            ));
        }
    };
    Ok(ApiCookie {
        name,
        value,
        domain,
        path,
        expires,
        http_only,
        secure,
        same_site,
    })
}

fn cookie_to_python(cookie: ApiCookie) -> PythonCookie {
    let same_site = match cookie.same_site {
        ApiSameSite::Strict => "Strict",
        ApiSameSite::Lax => "Lax",
        ApiSameSite::None => "None",
    };
    (
        cookie.name,
        cookie.value,
        cookie.domain,
        cookie.path,
        cookie.expires,
        cookie.http_only,
        cookie.secure,
        same_site.to_owned(),
    )
}

fn bounded_u16(value: u32, field: &str) -> PyResult<u16> {
    u16::try_from(value).map_err(|_| Error::new_err(format!("{field} must be between 0 and 65535")))
}

fn api_request_error(error: ApiRequestError) -> PyErr {
    display_error(error)
}

fn display_error(error: impl std::fmt::Display) -> PyErr {
    Error::new_err(error.to_string())
}
