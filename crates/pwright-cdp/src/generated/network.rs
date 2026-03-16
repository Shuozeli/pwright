//! CDP `Network` domain — generated from protocol JSON.
//!
//! Network domain allows tracking network activities of the page. It exposes information about http,
//! file, data and other requests and responses, their headers, bodies, timing, etc.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Resource type as it was perceived by the rendering engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    Document,
    Stylesheet,
    Image,
    Media,
    Font,
    Script,
    TextTrack,
    XHR,
    Fetch,
    Prefetch,
    EventSource,
    WebSocket,
    Manifest,
    SignedExchange,
    Ping,
    CSPViolationReport,
    Preflight,
    FedCM,
    Other,
}

/// Unique loader identifier.
pub type LoaderId = String;

/// Unique network request identifier.
/// Note that this does not identify individual HTTP requests that are part of
/// a network request.
pub type RequestId = String;

/// Unique intercepted request identifier.
pub type InterceptionId = String;

/// Network level fetch failure reason.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorReason {
    Failed,
    Aborted,
    TimedOut,
    AccessDenied,
    ConnectionClosed,
    ConnectionReset,
    ConnectionRefused,
    ConnectionAborted,
    ConnectionFailed,
    NameNotResolved,
    InternetDisconnected,
    AddressUnreachable,
    BlockedByClient,
    BlockedByResponse,
}

/// UTC time in seconds, counted from January 1, 1970.
pub type TimeSinceEpoch = f64;

/// Monotonically increasing time in seconds since an arbitrary point in the past.
pub type MonotonicTime = f64;

/// Request / response headers as keys / values of JSON object.
pub type Headers = Value;

/// The underlying connection technology that the browser is supposedly using.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "cellular2g")]
    Cellular2g,
    #[serde(rename = "cellular3g")]
    Cellular3g,
    #[serde(rename = "cellular4g")]
    Cellular4g,
    #[serde(rename = "bluetooth")]
    Bluetooth,
    #[serde(rename = "ethernet")]
    Ethernet,
    #[serde(rename = "wifi")]
    Wifi,
    #[serde(rename = "wimax")]
    Wimax,
    #[serde(rename = "other")]
    Other,
}

/// Represents the cookie's 'SameSite' status:
/// <https://tools.ietf.org/html/draft-west-first-party-cookies>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieSameSite {
    Strict,
    Lax,
    None,
}

/// Represents the cookie's 'Priority' status:
/// <https://tools.ietf.org/html/draft-west-cookie-priority-00>
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookiePriority {
    Low,
    Medium,
    High,
}

/// Represents the source scheme of the origin that originally set the cookie.
/// A value of "Unset" allows protocol clients to emulate legacy cookie scope for the scheme.
/// This is a temporary ability and it will be removed in the future.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieSourceScheme {
    Unset,
    NonSecure,
    Secure,
}

/// Timing information for the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTiming {
    /// Timing's requestTime is a baseline in seconds, while the other numbers are ticks in
    /// milliseconds relatively to this requestTime.
    pub request_time: f64,
    /// Started resolving proxy.
    pub proxy_start: f64,
    /// Finished resolving proxy.
    pub proxy_end: f64,
    /// Started DNS address resolve.
    pub dns_start: f64,
    /// Finished DNS address resolve.
    pub dns_end: f64,
    /// Started connecting to the remote host.
    pub connect_start: f64,
    /// Connected to the remote host.
    pub connect_end: f64,
    /// Started SSL handshake.
    pub ssl_start: f64,
    /// Finished SSL handshake.
    pub ssl_end: f64,
    /// Started running ServiceWorker.
    pub worker_start: f64,
    /// Finished Starting ServiceWorker.
    pub worker_ready: f64,
    /// Started fetch event.
    pub worker_fetch_start: f64,
    /// Settled fetch event respondWith promise.
    pub worker_respond_with_settled: f64,
    /// Started ServiceWorker static routing source evaluation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_router_evaluation_start: Option<f64>,
    /// Started cache lookup when the source was evaluated to `cache`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_cache_lookup_start: Option<f64>,
    /// Started sending request.
    pub send_start: f64,
    /// Finished sending request.
    pub send_end: f64,
    /// Time the server started pushing request.
    pub push_start: f64,
    /// Time the server finished pushing request.
    pub push_end: f64,
    /// Started receiving response headers.
    pub receive_headers_start: f64,
    /// Finished receiving response headers.
    pub receive_headers_end: f64,
}

/// Loading priority of a resource request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourcePriority {
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

/// Post data entry for HTTP request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PostDataEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<String>,
}

/// HTTP request data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// Request URL (without fragment).
    pub url: String,
    /// Fragment of the requested URL starting with hash, if present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_fragment: Option<String>,
    /// HTTP request method.
    pub method: String,
    /// HTTP request headers.
    pub headers: Headers,
    /// HTTP POST request data.
    /// Use postDataEntries instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_data: Option<String>,
    /// True when the request has POST data. Note that postData might still be omitted when this flag is true when the data is too long.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_post_data: Option<bool>,
    /// Request body elements (post data broken into individual entries).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_data_entries: Option<Vec<PostDataEntry>>,
    /// The mixed content type of the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mixed_content_type: Option<Value>,
    /// Priority of the resource request at the time request is sent.
    pub initial_priority: ResourcePriority,
    /// The referrer policy of the request, as defined in <https://www.w3.org/TR/referrer-policy/>
    pub referrer_policy: String,
    /// Whether is loaded via link preload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_link_preload: Option<bool>,
    /// Set for requests when the TrustToken API is used. Contains the parameters
    /// passed by the developer (e.g. via "fetch") as understood by the backend.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_token_params: Option<TrustTokenParams>,
    /// True if this resource request is considered to be the 'same site' as the
    /// request corresponding to the main frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_same_site: Option<bool>,
    /// True when the resource request is ad-related.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_ad_related: Option<bool>,
}

/// Details of a signed certificate timestamp (SCT).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedCertificateTimestamp {
    /// Validation status.
    pub status: String,
    /// Origin.
    pub origin: String,
    /// Log name / description.
    pub log_description: String,
    /// Log ID.
    pub log_id: String,
    /// Issuance date. Unlike TimeSinceEpoch, this contains the number of
    /// milliseconds since January 1, 1970, UTC, not the number of seconds.
    pub timestamp: f64,
    /// Hash algorithm.
    pub hash_algorithm: String,
    /// Signature algorithm.
    pub signature_algorithm: String,
    /// Signature data.
    pub signature_data: String,
}

/// Security details about a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityDetails {
    /// Protocol name (e.g. "TLS 1.2" or "QUIC").
    pub protocol: String,
    /// Key Exchange used by the connection, or the empty string if not applicable.
    pub key_exchange: String,
    /// (EC)DH group used by the connection, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_exchange_group: Option<String>,
    /// Cipher name.
    pub cipher: String,
    /// TLS MAC. Note that AEAD ciphers do not have separate MACs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mac: Option<String>,
    /// Certificate ID value.
    pub certificate_id: Value,
    /// Certificate subject name.
    pub subject_name: String,
    /// Subject Alternative Name (SAN) DNS names and IP addresses.
    pub san_list: Vec<String>,
    /// Name of the issuing CA.
    pub issuer: String,
    /// Certificate valid from date.
    pub valid_from: TimeSinceEpoch,
    /// Certificate valid to (expiration) date
    pub valid_to: TimeSinceEpoch,
    /// List of signed certificate timestamps (SCTs).
    pub signed_certificate_timestamp_list: Vec<SignedCertificateTimestamp>,
    /// Whether the request complied with Certificate Transparency policy
    pub certificate_transparency_compliance: CertificateTransparencyCompliance,
    /// The signature algorithm used by the server in the TLS server signature,
    /// represented as a TLS SignatureScheme code point. Omitted if not
    /// applicable or not known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_signature_algorithm: Option<i64>,
    /// Whether the connection used Encrypted ClientHello
    pub encrypted_client_hello: bool,
}

/// Whether the request complied with Certificate Transparency policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CertificateTransparencyCompliance {
    #[serde(rename = "unknown")]
    Unknown,
    #[serde(rename = "not-compliant")]
    NotCompliant,
    #[serde(rename = "compliant")]
    Compliant,
}

/// The reason why request was blocked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BlockedReason {
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "csp")]
    Csp,
    #[serde(rename = "mixed-content")]
    MixedContent,
    #[serde(rename = "origin")]
    Origin,
    #[serde(rename = "inspector")]
    Inspector,
    #[serde(rename = "integrity")]
    Integrity,
    #[serde(rename = "subresource-filter")]
    SubresourceFilter,
    #[serde(rename = "content-type")]
    ContentType,
    #[serde(rename = "coep-frame-resource-needs-coep-header")]
    CoepFrameResourceNeedsCoepHeader,
    #[serde(rename = "coop-sandboxed-iframe-cannot-navigate-to-coop-page")]
    CoopSandboxedIframeCannotNavigateToCoopPage,
    #[serde(rename = "corp-not-same-origin")]
    CorpNotSameOrigin,
    #[serde(rename = "corp-not-same-origin-after-defaulted-to-same-origin-by-coep")]
    CorpNotSameOriginAfterDefaultedToSameOriginByCoep,
    #[serde(rename = "corp-not-same-origin-after-defaulted-to-same-origin-by-dip")]
    CorpNotSameOriginAfterDefaultedToSameOriginByDip,
    #[serde(rename = "corp-not-same-origin-after-defaulted-to-same-origin-by-coep-and-dip")]
    CorpNotSameOriginAfterDefaultedToSameOriginByCoepAndDip,
    #[serde(rename = "corp-not-same-site")]
    CorpNotSameSite,
    #[serde(rename = "sri-message-signature-mismatch")]
    SriMessageSignatureMismatch,
}

/// The reason why request was blocked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CorsError {
    DisallowedByMode,
    InvalidResponse,
    WildcardOriginNotAllowed,
    MissingAllowOriginHeader,
    MultipleAllowOriginValues,
    InvalidAllowOriginValue,
    AllowOriginMismatch,
    InvalidAllowCredentials,
    CorsDisabledScheme,
    PreflightInvalidStatus,
    PreflightDisallowedRedirect,
    PreflightWildcardOriginNotAllowed,
    PreflightMissingAllowOriginHeader,
    PreflightMultipleAllowOriginValues,
    PreflightInvalidAllowOriginValue,
    PreflightAllowOriginMismatch,
    PreflightInvalidAllowCredentials,
    PreflightMissingAllowExternal,
    PreflightInvalidAllowExternal,
    PreflightMissingAllowPrivateNetwork,
    PreflightInvalidAllowPrivateNetwork,
    InvalidAllowMethodsPreflightResponse,
    InvalidAllowHeadersPreflightResponse,
    MethodDisallowedByPreflightResponse,
    HeaderDisallowedByPreflightResponse,
    RedirectContainsCredentials,
    InsecurePrivateNetwork,
    InvalidPrivateNetworkAccess,
    UnexpectedPrivateNetworkAccess,
    NoCorsRedirectModeNotFollow,
    PreflightMissingPrivateNetworkAccessId,
    PreflightMissingPrivateNetworkAccessName,
    PrivateNetworkAccessPermissionUnavailable,
    PrivateNetworkAccessPermissionDenied,
    LocalNetworkAccessPermissionDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorsErrorStatus {
    pub cors_error: CorsError,
    pub failed_parameter: String,
}

/// Source of serviceworker response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceWorkerResponseSource {
    #[serde(rename = "cache-storage")]
    CacheStorage,
    #[serde(rename = "http-cache")]
    HttpCache,
    #[serde(rename = "fallback-code")]
    FallbackCode,
    #[serde(rename = "network")]
    Network,
}

/// Determines what type of Trust Token operation is executed and
/// depending on the type, some additional parameters. The values
/// are specified in third_party/blink/renderer/core/fetch/trust_token.idl.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustTokenParams {
    pub operation: TrustTokenOperationType,
    /// Only set for "token-redemption" operation and determine whether
    /// to request a fresh SRR or use a still valid cached SRR.
    pub refresh_policy: String,
    /// Origins of issuers from whom to request tokens or redemption
    /// records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustTokenOperationType {
    Issuance,
    Redemption,
    Signing,
}

/// The reason why Chrome uses a specific transport protocol for HTTP semantics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AlternateProtocolUsage {
    #[serde(rename = "alternativeJobWonWithoutRace")]
    AlternativeJobWonWithoutRace,
    #[serde(rename = "alternativeJobWonRace")]
    AlternativeJobWonRace,
    #[serde(rename = "mainJobWonRace")]
    MainJobWonRace,
    #[serde(rename = "mappingMissing")]
    MappingMissing,
    #[serde(rename = "broken")]
    Broken,
    #[serde(rename = "dnsAlpnH3JobWonWithoutRace")]
    DnsAlpnH3JobWonWithoutRace,
    #[serde(rename = "dnsAlpnH3JobWonRace")]
    DnsAlpnH3JobWonRace,
    #[serde(rename = "unspecifiedReason")]
    UnspecifiedReason,
}

/// Source of service worker router.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceWorkerRouterSource {
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "cache")]
    Cache,
    #[serde(rename = "fetch-event")]
    FetchEvent,
    #[serde(rename = "race-network-and-fetch-handler")]
    RaceNetworkAndFetchHandler,
    #[serde(rename = "race-network-and-cache")]
    RaceNetworkAndCache,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceWorkerRouterInfo {
    /// ID of the rule matched. If there is a matched rule, this field will
    /// be set, otherwiser no value will be set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id_matched: Option<i64>,
    /// The router source of the matched rule. If there is a matched rule, this
    /// field will be set, otherwise no value will be set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_source_type: Option<ServiceWorkerRouterSource>,
    /// The actual router source used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_source_type: Option<ServiceWorkerRouterSource>,
}

/// HTTP response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    /// Response URL. This URL can be different from CachedResource.url in case of redirect.
    pub url: String,
    /// HTTP response status code.
    pub status: i64,
    /// HTTP response status text.
    pub status_text: String,
    /// HTTP response headers.
    pub headers: Headers,
    /// HTTP response headers text. This has been replaced by the headers in Network.responseReceivedExtraInfo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_text: Option<String>,
    /// Resource mimeType as determined by the browser.
    pub mime_type: String,
    /// Resource charset as determined by the browser (if applicable).
    pub charset: String,
    /// Refined HTTP request headers that were actually transmitted over the network.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<Headers>,
    /// HTTP request headers text. This has been replaced by the headers in Network.requestWillBeSentExtraInfo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_headers_text: Option<String>,
    /// Specifies whether physical connection was actually reused for this request.
    pub connection_reused: bool,
    /// Physical connection id that was actually used for this request.
    pub connection_id: f64,
    /// Remote IP address.
    #[serde(rename = "remoteIPAddress")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_ip_address: Option<String>,
    /// Remote port.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<i64>,
    /// Specifies that the request was served from the disk cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_disk_cache: Option<bool>,
    /// Specifies that the request was served from the ServiceWorker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_service_worker: Option<bool>,
    /// Specifies that the request was served from the prefetch cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_prefetch_cache: Option<bool>,
    /// Specifies that the request was served from the prefetch cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_early_hints: Option<bool>,
    /// Information about how ServiceWorker Static Router API was used. If this
    /// field is set with `matchedSourceType` field, a matching rule is found.
    /// If this field is set without `matchedSource`, no matching rule is found.
    /// Otherwise, the API is not used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_worker_router_info: Option<ServiceWorkerRouterInfo>,
    /// Total number of bytes received for this request so far.
    pub encoded_data_length: f64,
    /// Timing information for the given request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<ResourceTiming>,
    /// Response source of response from ServiceWorker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_worker_response_source: Option<ServiceWorkerResponseSource>,
    /// The time at which the returned response was generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_time: Option<TimeSinceEpoch>,
    /// Cache Storage Cache Name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_storage_cache_name: Option<String>,
    /// Protocol used to fetch this request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// The reason why Chrome uses a specific transport protocol for HTTP semantics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternate_protocol_usage: Option<AlternateProtocolUsage>,
    /// Security state of the request resource.
    pub security_state: Value,
    /// Security details for the request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_details: Option<SecurityDetails>,
}

/// WebSocket request data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketRequest {
    /// HTTP request headers.
    pub headers: Headers,
}

/// WebSocket response data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketResponse {
    /// HTTP response status code.
    pub status: i64,
    /// HTTP response status text.
    pub status_text: String,
    /// HTTP response headers.
    pub headers: Headers,
    /// HTTP response headers text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers_text: Option<String>,
    /// HTTP request headers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_headers: Option<Headers>,
    /// HTTP request headers text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_headers_text: Option<String>,
}

/// WebSocket message data. This represents an entire WebSocket message, not just a fragmented frame as the name suggests.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketFrame {
    /// WebSocket message opcode.
    pub opcode: f64,
    /// WebSocket message mask.
    pub mask: bool,
    /// WebSocket message payload data.
    /// If the opcode is 1, this is a text message and payloadData is a UTF-8 string.
    /// If the opcode isn't 1, then payloadData is a base64 encoded string representing binary data.
    pub payload_data: String,
}

/// Information about the cached resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CachedResource {
    /// Resource URL. This is the url of the original network request.
    pub url: String,
    /// Type of this resource.
    #[serde(rename = "type")]
    pub r#type: ResourceType,
    /// Cached response data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,
    /// Cached response body size.
    pub body_size: f64,
}

/// Information about the request initiator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Initiator {
    /// Type of this initiator.
    #[serde(rename = "type")]
    pub r#type: String,
    /// Initiator JavaScript stack trace, set for Script only.
    /// Requires the Debugger domain to be enabled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<super::runtime::StackTrace>,
    /// Initiator URL, set for Parser type or for Script type (when script is importing module) or for SignedExchange type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Initiator line number, set for Parser type or for Script type (when script is importing
    /// module) (0-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<f64>,
    /// Initiator column number, set for Parser type or for Script type (when script is importing
    /// module) (0-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_number: Option<f64>,
    /// Set if another request triggered this request (e.g. preflight).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

/// cookiePartitionKey object
/// The representation of the components of the key that are created by the cookiePartitionKey class contained in net/cookies/cookie_partition_key.h.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookiePartitionKey {
    /// The site of the top-level URL the browser was visiting at the start
    /// of the request to the endpoint that set the cookie.
    pub top_level_site: String,
    /// Indicates if the cookie has any ancestors that are cross-site to the topLevelSite.
    pub has_cross_site_ancestor: bool,
}

/// Cookie object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// Cookie domain.
    pub domain: String,
    /// Cookie path.
    pub path: String,
    /// Cookie expiration date as the number of seconds since the UNIX epoch.
    /// The value is set to -1 if the expiry date is not set.
    /// The value can be null for values that cannot be represented in
    /// JSON (±Inf).
    pub expires: f64,
    /// Cookie size.
    pub size: i64,
    /// True if cookie is http-only.
    pub http_only: bool,
    /// True if cookie is secure.
    pub secure: bool,
    /// True in case of session cookie.
    pub session: bool,
    /// Cookie SameSite type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<CookieSameSite>,
    /// Cookie Priority
    pub priority: CookiePriority,
    /// True if cookie is SameParty.
    pub same_party: bool,
    /// Cookie source scheme type.
    pub source_scheme: CookieSourceScheme,
    /// Cookie source port. Valid values are {-1, \[1, 65535\]}, -1 indicates an unspecified port.
    /// An unspecified port value allows protocol clients to emulate legacy cookie scope for the port.
    /// This is a temporary ability and it will be removed in the future.
    pub source_port: i64,
    /// Cookie partition key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<CookiePartitionKey>,
    /// True if cookie partition key is opaque.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_key_opaque: Option<bool>,
}

/// Types of reasons why a cookie may not be stored from a response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SetCookieBlockedReason {
    SecureOnly,
    SameSiteStrict,
    SameSiteLax,
    SameSiteUnspecifiedTreatedAsLax,
    SameSiteNoneInsecure,
    UserPreferences,
    ThirdPartyPhaseout,
    ThirdPartyBlockedInFirstPartySet,
    SyntaxError,
    SchemeNotSupported,
    OverwriteSecure,
    InvalidDomain,
    InvalidPrefix,
    UnknownError,
    SchemefulSameSiteStrict,
    SchemefulSameSiteLax,
    SchemefulSameSiteUnspecifiedTreatedAsLax,
    SamePartyFromCrossPartyContext,
    SamePartyConflictsWithOtherAttributes,
    NameValuePairExceedsMaxSize,
    DisallowedCharacter,
    NoCookieContent,
}

/// Types of reasons why a cookie may not be sent with a request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieBlockedReason {
    SecureOnly,
    NotOnPath,
    DomainMismatch,
    SameSiteStrict,
    SameSiteLax,
    SameSiteUnspecifiedTreatedAsLax,
    SameSiteNoneInsecure,
    UserPreferences,
    ThirdPartyPhaseout,
    ThirdPartyBlockedInFirstPartySet,
    UnknownError,
    SchemefulSameSiteStrict,
    SchemefulSameSiteLax,
    SchemefulSameSiteUnspecifiedTreatedAsLax,
    SamePartyFromCrossPartyContext,
    NameValuePairExceedsMaxSize,
    PortMismatch,
    SchemeMismatch,
    AnonymousContext,
}

/// Types of reasons why a cookie should have been blocked by 3PCD but is exempted for the request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CookieExemptionReason {
    None,
    UserSetting,
    TPCDMetadata,
    TPCDDeprecationTrial,
    TopLevelTPCDDeprecationTrial,
    TPCDHeuristics,
    EnterprisePolicy,
    StorageAccess,
    TopLevelStorageAccess,
    Scheme,
    SameSiteNoneCookiesInSandbox,
}

/// A cookie which was not stored from a response with the corresponding reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockedSetCookieWithReason {
    /// The reason(s) this cookie was blocked.
    pub blocked_reasons: Vec<SetCookieBlockedReason>,
    /// The string representing this individual cookie as it would appear in the header.
    /// This is not the entire "cookie" or "set-cookie" header which could have multiple cookies.
    pub cookie_line: String,
    /// The cookie object which represents the cookie which was not stored. It is optional because
    /// sometimes complete cookie information is not available, such as in the case of parsing
    /// errors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookie: Option<Cookie>,
}

/// A cookie should have been blocked by 3PCD but is exempted and stored from a response with the
/// corresponding reason. A cookie could only have at most one exemption reason.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExemptedSetCookieWithReason {
    /// The reason the cookie was exempted.
    pub exemption_reason: CookieExemptionReason,
    /// The string representing this individual cookie as it would appear in the header.
    pub cookie_line: String,
    /// The cookie object representing the cookie.
    pub cookie: Cookie,
}

/// A cookie associated with the request which may or may not be sent with it.
/// Includes the cookies itself and reasons for blocking or exemption.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssociatedCookie {
    /// The cookie object representing the cookie which was not sent.
    pub cookie: Cookie,
    /// The reason(s) the cookie was blocked. If empty means the cookie is included.
    pub blocked_reasons: Vec<CookieBlockedReason>,
    /// The reason the cookie should have been blocked by 3PCD but is exempted. A cookie could
    /// only have at most one exemption reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exemption_reason: Option<CookieExemptionReason>,
}

/// Cookie parameter object
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieParam {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// The request-URI to associate with the setting of the cookie. This value can affect the
    /// default domain, path, source port, and source scheme values of the created cookie.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Cookie domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Cookie path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// True if cookie is secure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
    /// True if cookie is http-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    /// Cookie SameSite type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<CookieSameSite>,
    /// Cookie expiration date, session cookie if not set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires: Option<TimeSinceEpoch>,
    /// Cookie Priority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<CookiePriority>,
    /// True if cookie is SameParty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_party: Option<bool>,
    /// Cookie source scheme type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_scheme: Option<CookieSourceScheme>,
    /// Cookie source port. Valid values are {-1, \[1, 65535\]}, -1 indicates an unspecified port.
    /// An unspecified port value allows protocol clients to emulate legacy cookie scope for the port.
    /// This is a temporary ability and it will be removed in the future.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_port: Option<i64>,
    /// Cookie partition key. If not set, the cookie will be set as not partitioned.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<CookiePartitionKey>,
}

/// Authorization challenge for HTTP status code 401 or 407.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallenge {
    /// Source of the authentication challenge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Origin of the challenger.
    pub origin: String,
    /// The authentication scheme used, such as basic or digest
    pub scheme: String,
    /// The realm of the challenge. May be empty.
    pub realm: String,
}

/// Response to an AuthChallenge.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallengeResponse {
    /// The decision on what to do in response to the authorization challenge.  Default means
    /// deferring to the default behavior of the net stack, which will likely either the Cancel
    /// authentication or display a popup dialog box.
    pub response: String,
    /// The username to provide, possibly empty. Should only be set if response is
    /// ProvideCredentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// The password to provide, possibly empty. Should only be set if response is
    /// ProvideCredentials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

/// Stages of the interception to begin intercepting. Request will intercept before the request is
/// sent. Response will intercept after the response is received.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterceptionStage {
    Request,
    HeadersReceived,
}

/// Request pattern for interception.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequestPattern {
    /// Wildcards (`'*'` -> zero or more, `'?'` -> exactly one) are allowed. Escape character is
    /// backslash. Omitting is equivalent to `"*"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    /// If set, only requests for matching resource types will be intercepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<ResourceType>,
    /// Stage at which to begin intercepting requests. Default is Request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interception_stage: Option<InterceptionStage>,
}

/// Information about a signed exchange signature.
/// <https://wicg.github.io/webpackage/draft-yasskin-httpbis-origin-signed-exchanges-impl.html#rfc.section.3.1>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedExchangeSignature {
    /// Signed exchange signature label.
    pub label: String,
    /// The hex string of signed exchange signature.
    pub signature: String,
    /// Signed exchange signature integrity.
    pub integrity: String,
    /// Signed exchange signature cert Url.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_url: Option<String>,
    /// The hex string of signed exchange signature cert sha256.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_sha256: Option<String>,
    /// Signed exchange signature validity Url.
    pub validity_url: String,
    /// Signed exchange signature date.
    pub date: i64,
    /// Signed exchange signature expires.
    pub expires: i64,
    /// The encoded certificates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificates: Option<Vec<String>>,
}

/// Information about a signed exchange header.
/// <https://wicg.github.io/webpackage/draft-yasskin-httpbis-origin-signed-exchanges-impl.html#cbor-representation>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedExchangeHeader {
    /// Signed exchange request URL.
    pub request_url: String,
    /// Signed exchange response code.
    pub response_code: i64,
    /// Signed exchange response headers.
    pub response_headers: Headers,
    /// Signed exchange response signature.
    pub signatures: Vec<SignedExchangeSignature>,
    /// Signed exchange header integrity hash in the form of `sha256-<base64-hash-value>`.
    pub header_integrity: String,
}

/// Field type for a signed exchange related error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SignedExchangeErrorField {
    #[serde(rename = "signatureSig")]
    SignatureSig,
    #[serde(rename = "signatureIntegrity")]
    SignatureIntegrity,
    #[serde(rename = "signatureCertUrl")]
    SignatureCertUrl,
    #[serde(rename = "signatureCertSha256")]
    SignatureCertSha256,
    #[serde(rename = "signatureValidityUrl")]
    SignatureValidityUrl,
    #[serde(rename = "signatureTimestamps")]
    SignatureTimestamps,
}

/// Information about a signed exchange response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedExchangeError {
    /// Error message.
    pub message: String,
    /// The index of the signature which caused the error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_index: Option<i64>,
    /// The field which caused the error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_field: Option<SignedExchangeErrorField>,
}

/// Information about a signed exchange response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedExchangeInfo {
    /// The outer response of signed HTTP exchange which was received from network.
    pub outer_response: Response,
    /// Whether network response for the signed exchange was accompanied by
    /// extra headers.
    pub has_extra_info: bool,
    /// Information about the signed exchange header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<SignedExchangeHeader>,
    /// Security details for the signed exchange header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_details: Option<SecurityDetails>,
    /// Errors occurred while handling the signed exchange.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<SignedExchangeError>>,
}

/// List of content encodings supported by the backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentEncoding {
    #[serde(rename = "deflate")]
    Deflate,
    #[serde(rename = "gzip")]
    Gzip,
    #[serde(rename = "br")]
    Br,
    #[serde(rename = "zstd")]
    Zstd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConditions {
    /// Only matching requests will be affected by these conditions. Patterns use the URLPattern constructor string
    /// syntax (<https://urlpattern.spec.whatwg.org/>) and must be absolute. If the pattern is empty, all requests are
    /// matched (including p2p connections).
    pub url_pattern: String,
    /// Minimum latency from request sent to response headers received (ms).
    pub latency: f64,
    /// Maximal aggregated download throughput (bytes/sec). -1 disables download throttling.
    pub download_throughput: f64,
    /// Maximal aggregated upload throughput (bytes/sec).  -1 disables upload throttling.
    pub upload_throughput: f64,
    /// Connection type if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
    /// WebRTC packet loss (percent, 0-100). 0 disables packet loss emulation, 100 drops all the packets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f64>,
    /// WebRTC packet queue length (packet). 0 removes any queue length limitations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_queue_length: Option<i64>,
    /// WebRTC packetReordering feature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub packet_reordering: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockPattern {
    /// URL pattern to match. Patterns use the URLPattern constructor string syntax
    /// (<https://urlpattern.spec.whatwg.org/>) and must be absolute. Example: `*://*:*/*.css`.
    pub url_pattern: String,
    /// Whether or not to block the pattern. If false, a matching request will not be blocked even if it matches a later
    /// `BlockPattern`.
    pub block: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DirectSocketDnsQueryType {
    #[serde(rename = "ipv4")]
    Ipv4,
    #[serde(rename = "ipv6")]
    Ipv6,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectTCPSocketOptions {
    /// TCP_NODELAY option
    pub no_delay: bool,
    /// Expected to be unsigned integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keep_alive_delay: Option<f64>,
    /// Expected to be unsigned integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_buffer_size: Option<f64>,
    /// Expected to be unsigned integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receive_buffer_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns_query_type: Option<DirectSocketDnsQueryType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_addr: Option<String>,
    /// Unsigned int 16.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_addr: Option<String>,
    /// Unsigned int 16.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_port: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dns_query_type: Option<DirectSocketDnsQueryType>,
    /// Expected to be unsigned integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub send_buffer_size: Option<f64>,
    /// Expected to be unsigned integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receive_buffer_size: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multicast_loopback: Option<bool>,
    /// Unsigned int 8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multicast_time_to_live: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multicast_allow_address_sharing: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPMessage {
    pub data: String,
    /// Null for connected mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_addr: Option<String>,
    /// Null for connected mode.
    /// Expected to be unsigned integer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_port: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivateNetworkRequestPolicy {
    Allow,
    BlockFromInsecureToMorePrivate,
    WarnFromInsecureToMorePrivate,
    PermissionBlock,
    PermissionWarn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IPAddressSpace {
    Loopback,
    Local,
    Public,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectTiming {
    /// Timing's requestTime is a baseline in seconds, while the other numbers are ticks in
    /// milliseconds relatively to this requestTime. Matches ResourceTiming's requestTime for
    /// the same request (but not for redirected requests).
    pub request_time: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSecurityState {
    pub initiator_is_secure_context: bool,
    #[serde(rename = "initiatorIPAddressSpace")]
    pub initiator_ip_address_space: IPAddressSpace,
    pub private_network_request_policy: PrivateNetworkRequestPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrossOriginOpenerPolicyValue {
    SameOrigin,
    SameOriginAllowPopups,
    RestrictProperties,
    UnsafeNone,
    SameOriginPlusCoep,
    RestrictPropertiesPlusCoep,
    NoopenerAllowPopups,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossOriginOpenerPolicyStatus {
    pub value: CrossOriginOpenerPolicyValue,
    pub report_only_value: CrossOriginOpenerPolicyValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporting_endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_only_reporting_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrossOriginEmbedderPolicyValue {
    None,
    Credentialless,
    RequireCorp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossOriginEmbedderPolicyStatus {
    pub value: CrossOriginEmbedderPolicyValue,
    pub report_only_value: CrossOriginEmbedderPolicyValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reporting_endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_only_reporting_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentSecurityPolicySource {
    HTTP,
    Meta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSecurityPolicyStatus {
    pub effective_directives: String,
    pub is_enforced: bool,
    pub source: ContentSecurityPolicySource,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecurityIsolationStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coop: Option<CrossOriginOpenerPolicyStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coep: Option<CrossOriginEmbedderPolicyStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub csp: Option<Vec<ContentSecurityPolicyStatus>>,
}

/// The status of a Reporting API report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReportStatus {
    Queued,
    Pending,
    MarkedForRemoval,
    Success,
}

pub type ReportId = String;

/// An object representing a report generated by the Reporting API.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingApiReport {
    pub id: ReportId,
    /// The URL of the document that triggered the report.
    pub initiator_url: String,
    /// The name of the endpoint group that should be used to deliver the report.
    pub destination: String,
    /// The type of the report (specifies the set of data that is contained in the report body).
    #[serde(rename = "type")]
    pub r#type: String,
    /// When the report was generated.
    pub timestamp: super::network::TimeSinceEpoch,
    /// How many uploads deep the related request was.
    pub depth: i64,
    /// The number of delivery attempts made so far, not including an active attempt.
    pub completed_attempts: i64,
    pub body: Value,
    pub status: ReportStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingApiEndpoint {
    /// The URL of the endpoint to which reports may be delivered.
    pub url: String,
    /// Name of the endpoint group.
    pub group_name: String,
}

/// An object providing the result of a network resource load.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadNetworkResourcePageResult {
    pub success: bool,
    /// Optional values used for error reporting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_error: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_error_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status_code: Option<f64>,
    /// If successful, one of the following two fields holds the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<Value>,
    /// Response headers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub headers: Option<super::network::Headers>,
}

/// An options object that may be extended later to better support CORS,
/// CORB and streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadNetworkResourceOptions {
    pub disable_cache: bool,
    pub include_credentials: bool,
}

/// Parameters for `Network.setAcceptedEncodings`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetAcceptedEncodingsParams {
    /// List of accepted content encodings.
    pub encodings: Vec<ContentEncoding>,
}

/// Return type for `Network.canClearBrowserCache`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanClearBrowserCacheReturns {
    /// True if browser cache can be cleared.
    pub result: bool,
}

/// Return type for `Network.canClearBrowserCookies`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanClearBrowserCookiesReturns {
    /// True if browser cookies can be cleared.
    pub result: bool,
}

/// Return type for `Network.canEmulateNetworkConditions`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanEmulateNetworkConditionsReturns {
    /// True if emulation of network conditions is supported.
    pub result: bool,
}

/// Parameters for `Network.continueInterceptedRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueInterceptedRequestParams {
    pub interception_id: InterceptionId,
    /// If set this causes the request to fail with the given reason. Passing `Aborted` for requests
    /// marked with `isNavigationRequest` also cancels the navigation. Must not be set in response
    /// to an authChallenge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<ErrorReason>,
    /// If set the requests completes using with the provided base64 encoded raw response, including
    /// HTTP status line and headers etc... Must not be set in response to an authChallenge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_response: Option<String>,
    /// If set the request url will be modified in a way that's not observable by page. Must not be
    /// set in response to an authChallenge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// If set this allows the request method to be overridden. Must not be set in response to an
    /// authChallenge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// If set this allows postData to be set. Must not be set in response to an authChallenge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_data: Option<String>,
    /// If set this allows the request headers to be changed. Must not be set in response to an
    /// authChallenge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Headers>,
    /// Response to a requestIntercepted with an authChallenge. Must not be set otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_challenge_response: Option<AuthChallengeResponse>,
}

/// Parameters for `Network.deleteCookies`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCookiesParams {
    /// Name of the cookies to remove.
    pub name: String,
    /// If specified, deletes all the cookies with the given name where domain and path match
    /// provided URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// If specified, deletes only cookies with the exact domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// If specified, deletes only cookies with the exact path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// If specified, deletes only cookies with the the given name and partitionKey where
    /// all partition key attributes match the cookie partition key attribute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<CookiePartitionKey>,
}

/// Parameters for `Network.emulateNetworkConditions`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmulateNetworkConditionsParams {
    /// True to emulate internet disconnection.
    pub offline: bool,
    /// Minimum latency from request sent to response headers received (ms).
    pub latency: f64,
    /// Maximal aggregated download throughput (bytes/sec). -1 disables download throttling.
    pub download_throughput: f64,
    /// Maximal aggregated upload throughput (bytes/sec).  -1 disables upload throttling.
    pub upload_throughput: f64,
    /// Connection type if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
    /// WebRTC packet loss (percent, 0-100). 0 disables packet loss emulation, 100 drops all the packets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_loss: Option<f64>,
    /// WebRTC packet queue length (packet). 0 removes any queue length limitations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_queue_length: Option<i64>,
    /// WebRTC packetReordering feature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packet_reordering: Option<bool>,
}

/// Parameters for `Network.emulateNetworkConditionsByRule`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EmulateNetworkConditionsByRuleParams {
    /// True to emulate internet disconnection.
    pub offline: bool,
    /// Configure conditions for matching requests. If multiple entries match a request, the first entry wins.  Global
    /// conditions can be configured by leaving the urlPattern for the conditions empty. These global conditions are
    /// also applied for throttling of p2p connections.
    pub matched_network_conditions: Vec<NetworkConditions>,
}

/// Return type for `Network.emulateNetworkConditionsByRule`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmulateNetworkConditionsByRuleReturns {
    /// An id for each entry in matchedNetworkConditions. The id will be included in the requestWillBeSentExtraInfo for
    /// requests affected by a rule.
    pub rule_ids: Vec<String>,
}

/// Parameters for `Network.overrideNetworkState`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OverrideNetworkStateParams {
    /// True to emulate internet disconnection.
    pub offline: bool,
    /// Minimum latency from request sent to response headers received (ms).
    pub latency: f64,
    /// Maximal aggregated download throughput (bytes/sec). -1 disables download throttling.
    pub download_throughput: f64,
    /// Maximal aggregated upload throughput (bytes/sec).  -1 disables upload throttling.
    pub upload_throughput: f64,
    /// Connection type if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<ConnectionType>,
}

/// Parameters for `Network.enable`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// Buffer size in bytes to use when preserving network payloads (XHRs, etc).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_total_buffer_size: Option<i64>,
    /// Per-resource buffer size in bytes to use when preserving network payloads (XHRs, etc).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_resource_buffer_size: Option<i64>,
    /// Longest post body size (in bytes) that would be included in requestWillBeSent notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_post_data_size: Option<i64>,
    /// Whether DirectSocket chunk send/receive events should be reported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_direct_socket_traffic: Option<bool>,
    /// Enable storing response bodies outside of renderer, so that these survive
    /// a cross-process navigation. Requires maxTotalBufferSize to be set.
    /// Currently defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_durable_messages: Option<bool>,
}

/// Return type for `Network.getAllCookies`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAllCookiesReturns {
    /// Array of cookie objects.
    pub cookies: Vec<Cookie>,
}

/// Parameters for `Network.getCertificate`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetCertificateParams {
    /// Origin to get certificate for.
    pub origin: String,
}

/// Return type for `Network.getCertificate`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCertificateReturns {
    pub table_names: Vec<String>,
}

/// Parameters for `Network.getCookies`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetCookiesParams {
    /// The list of URLs for which applicable cookies will be fetched.
    /// If not specified, it's assumed to be set to the list containing
    /// the URLs of the page and all of its subframes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,
}

/// Return type for `Network.getCookies`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCookiesReturns {
    /// Array of cookie objects.
    pub cookies: Vec<Cookie>,
}

/// Parameters for `Network.getResponseBody`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseBodyParams {
    /// Identifier of the network request to get content for.
    pub request_id: RequestId,
}

/// Return type for `Network.getResponseBody`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseBodyReturns {
    /// Response body.
    pub body: String,
    /// True, if content was sent as base64.
    pub base64_encoded: bool,
}

/// Parameters for `Network.getRequestPostData`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRequestPostDataParams {
    /// Identifier of the network request to get content for.
    pub request_id: RequestId,
}

/// Return type for `Network.getRequestPostData`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRequestPostDataReturns {
    /// Request body string, omitting files from multipart requests
    pub post_data: String,
}

/// Parameters for `Network.getResponseBodyForInterception`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseBodyForInterceptionParams {
    /// Identifier for the intercepted request to get body for.
    pub interception_id: InterceptionId,
}

/// Return type for `Network.getResponseBodyForInterception`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseBodyForInterceptionReturns {
    /// Response body.
    pub body: String,
    /// True, if content was sent as base64.
    pub base64_encoded: bool,
}

/// Parameters for `Network.takeResponseBodyForInterceptionAsStream`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeResponseBodyForInterceptionAsStreamParams {
    pub interception_id: InterceptionId,
}

/// Return type for `Network.takeResponseBodyForInterceptionAsStream`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeResponseBodyForInterceptionAsStreamReturns {
    pub stream: Value,
}

/// Parameters for `Network.replayXHR`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayXHRParams {
    /// Identifier of XHR to replay.
    pub request_id: RequestId,
}

/// Parameters for `Network.searchInResponseBody`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInResponseBodyParams {
    /// Identifier of the network response to search.
    pub request_id: RequestId,
    /// String to search for.
    pub query: String,
    /// If true, search is case sensitive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
    /// If true, treats string parameter as regex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_regex: Option<bool>,
}

/// Return type for `Network.searchInResponseBody`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInResponseBodyReturns {
    /// List of search matches.
    pub result: Vec<Value>,
}

/// Parameters for `Network.setBlockedURLs`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetBlockedURLsParams {
    /// Patterns to match in the order in which they are given. These patterns
    /// also take precedence over any wildcard patterns defined in `urls`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_patterns: Option<Vec<BlockPattern>>,
    /// URL patterns to block. Wildcards ('*') are allowed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,
}

/// Parameters for `Network.setBypassServiceWorker`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetBypassServiceWorkerParams {
    /// Bypass service worker and load from network.
    pub bypass: bool,
}

/// Parameters for `Network.setCacheDisabled`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetCacheDisabledParams {
    /// Cache disabled state.
    pub cache_disabled: bool,
}

/// Parameters for `Network.setCookie`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetCookieParams {
    /// Cookie name.
    pub name: String,
    /// Cookie value.
    pub value: String,
    /// The request-URI to associate with the setting of the cookie. This value can affect the
    /// default domain, path, source port, and source scheme values of the created cookie.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Cookie domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Cookie path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// True if cookie is secure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secure: Option<bool>,
    /// True if cookie is http-only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_only: Option<bool>,
    /// Cookie SameSite type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_site: Option<CookieSameSite>,
    /// Cookie expiration date, session cookie if not set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires: Option<TimeSinceEpoch>,
    /// Cookie Priority type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<CookiePriority>,
    /// True if cookie is SameParty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub same_party: Option<bool>,
    /// Cookie source scheme type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_scheme: Option<CookieSourceScheme>,
    /// Cookie source port. Valid values are {-1, \[1, 65535\]}, -1 indicates an unspecified port.
    /// An unspecified port value allows protocol clients to emulate legacy cookie scope for the port.
    /// This is a temporary ability and it will be removed in the future.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_port: Option<i64>,
    /// Cookie partition key. If not set, the cookie will be set as not partitioned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<CookiePartitionKey>,
}

/// Return type for `Network.setCookie`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetCookieReturns {
    /// Always set to true. If an error occurs, the response indicates protocol error.
    pub success: bool,
}

/// Parameters for `Network.setCookies`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetCookiesParams {
    /// Cookies to be set.
    pub cookies: Vec<CookieParam>,
}

/// Parameters for `Network.setExtraHTTPHeaders`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetExtraHTTPHeadersParams {
    /// Map with extra HTTP headers.
    pub headers: Headers,
}

/// Parameters for `Network.setAttachDebugStack`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetAttachDebugStackParams {
    /// Whether to attach a page script stack for debugging purpose.
    pub enabled: bool,
}

/// Parameters for `Network.setRequestInterception`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetRequestInterceptionParams {
    /// Requests matching any of these patterns will be forwarded and wait for the corresponding
    /// continueInterceptedRequest call.
    pub patterns: Vec<RequestPattern>,
}

/// Parameters for `Network.setUserAgentOverride`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetUserAgentOverrideParams {
    /// User agent to use.
    pub user_agent: String,
    /// Browser language to emulate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_language: Option<String>,
    /// The platform navigator.platform should return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    /// To be sent in Sec-CH-UA-* headers and returned in navigator.userAgentData
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent_metadata: Option<Value>,
}

/// Parameters for `Network.streamResourceContent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResourceContentParams {
    /// Identifier of the request to stream.
    pub request_id: RequestId,
}

/// Return type for `Network.streamResourceContent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamResourceContentReturns {
    /// Data that has been buffered until streaming is enabled.
    pub buffered_data: String,
}

/// Parameters for `Network.getSecurityIsolationStatus`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetSecurityIsolationStatusParams {
    /// If no frameId is provided, the status of the target is provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<super::page::FrameId>,
}

/// Return type for `Network.getSecurityIsolationStatus`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSecurityIsolationStatusReturns {
    pub status: SecurityIsolationStatus,
}

/// Parameters for `Network.enableReportingApi`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnableReportingApiParams {
    /// Whether to enable or disable events for the Reporting API
    pub enable: bool,
}

/// Parameters for `Network.loadNetworkResource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadNetworkResourceParams {
    /// Frame id to get the resource for. Mandatory for frame targets, and
    /// should be omitted for worker targets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<super::page::FrameId>,
    /// URL of the resource to get content for.
    pub url: String,
    /// Options for the request.
    pub options: LoadNetworkResourceOptions,
}

/// Return type for `Network.loadNetworkResource`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadNetworkResourceReturns {
    pub resource: LoadNetworkResourcePageResult,
}

/// Parameters for `Network.setCookieControls`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetCookieControlsParams {
    /// Whether 3pc restriction is enabled.
    pub enable_third_party_cookie_restriction: bool,
    /// Whether 3pc grace period exception should be enabled; false by default.
    pub disable_third_party_cookie_metadata: bool,
    /// Whether 3pc heuristics exceptions should be enabled; false by default.
    pub disable_third_party_cookie_heuristics: bool,
}

/// Event payload for `Network.dataReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataReceivedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// Data chunk length.
    pub data_length: i64,
    /// Actual bytes received (might be less than dataLength for compressed encodings).
    pub encoded_data_length: i64,
    /// Data that was received.
    #[serde(default)]
    pub data: Option<String>,
}

/// Event payload for `Network.eventSourceMessageReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventSourceMessageReceivedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// Message type.
    pub event_name: String,
    /// Message identifier.
    pub event_id: String,
    /// Message content.
    pub data: String,
}

/// Event payload for `Network.loadingFailed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadingFailedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// Resource type.
    #[serde(rename = "type")]
    pub r#type: ResourceType,
    /// Error message. List of network errors: <https://cs.chromium.org/chromium/src/net/base/net_error_list.h>
    pub error_text: String,
    /// True if loading was canceled.
    #[serde(default)]
    pub canceled: Option<bool>,
    /// The reason why loading was blocked, if any.
    #[serde(default)]
    pub blocked_reason: Option<BlockedReason>,
    /// The reason why loading was blocked by CORS, if any.
    #[serde(default)]
    pub cors_error_status: Option<CorsErrorStatus>,
}

/// Event payload for `Network.loadingFinished`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadingFinishedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// Total number of bytes received for this request.
    pub encoded_data_length: f64,
}

/// Event payload for `Network.requestIntercepted`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestInterceptedEvent {
    /// Each request the page makes will have a unique id, however if any redirects are encountered
    /// while processing that fetch, they will be reported with the same id as the original fetch.
    /// Likewise if HTTP authentication is needed then the same fetch id will be used.
    pub interception_id: InterceptionId,
    pub request: Request,
    /// The id of the frame that initiated the request.
    pub frame_id: super::page::FrameId,
    /// How the requested resource will be used.
    pub resource_type: ResourceType,
    /// Whether this is a navigation request, which can abort the navigation completely.
    pub is_navigation_request: bool,
    /// Set if the request is a navigation that will result in a download.
    /// Only present after response is received from the server (i.e. HeadersReceived stage).
    #[serde(default)]
    pub is_download: Option<bool>,
    /// Redirect location, only sent if a redirect was intercepted.
    #[serde(default)]
    pub redirect_url: Option<String>,
    /// Details of the Authorization Challenge encountered. If this is set then
    /// continueInterceptedRequest must contain an authChallengeResponse.
    #[serde(default)]
    pub auth_challenge: Option<AuthChallenge>,
    /// Response error if intercepted at response stage or if redirect occurred while intercepting
    /// request.
    #[serde(default)]
    pub response_error_reason: Option<ErrorReason>,
    /// Response code if intercepted at response stage or if redirect occurred while intercepting
    /// request or auth retry occurred.
    #[serde(default)]
    pub response_status_code: Option<i64>,
    /// Response headers if intercepted at the response stage or if redirect occurred while
    /// intercepting request or auth retry occurred.
    #[serde(default)]
    pub response_headers: Option<Headers>,
    /// If the intercepted request had a corresponding requestWillBeSent event fired for it, then
    /// this requestId will be the same as the requestId present in the requestWillBeSent event.
    #[serde(default)]
    pub request_id: Option<RequestId>,
}

/// Event payload for `Network.requestServedFromCache`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestServedFromCacheEvent {
    /// Request identifier.
    pub request_id: RequestId,
}

/// Event payload for `Network.requestWillBeSent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestWillBeSentEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Loader identifier. Empty string if the request is fetched from worker.
    pub loader_id: LoaderId,
    /// URL of the document this request is loaded for.
    #[serde(rename = "documentURL")]
    pub document_url: String,
    /// Request data.
    pub request: Request,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// Timestamp.
    pub wall_time: TimeSinceEpoch,
    /// Request initiator.
    pub initiator: Initiator,
    /// In the case that redirectResponse is populated, this flag indicates whether
    /// requestWillBeSentExtraInfo and responseReceivedExtraInfo events will be or were emitted
    /// for the request which was just redirected.
    pub redirect_has_extra_info: bool,
    /// Redirect response data.
    #[serde(default)]
    pub redirect_response: Option<Response>,
    /// Type of this resource.
    #[serde(rename = "type")]
    #[serde(default)]
    pub r#type: Option<ResourceType>,
    /// Frame identifier.
    #[serde(default)]
    pub frame_id: Option<super::page::FrameId>,
    /// Whether the request is initiated by a user gesture. Defaults to false.
    #[serde(default)]
    pub has_user_gesture: Option<bool>,
}

/// Event payload for `Network.resourceChangedPriority`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChangedPriorityEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// New priority
    pub new_priority: ResourcePriority,
    /// Timestamp.
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.signedExchangeReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedExchangeReceivedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Information about the signed exchange response.
    pub info: SignedExchangeInfo,
}

/// Event payload for `Network.responseReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseReceivedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Loader identifier. Empty string if the request is fetched from worker.
    pub loader_id: LoaderId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// Resource type.
    #[serde(rename = "type")]
    pub r#type: ResourceType,
    /// Response data.
    pub response: Response,
    /// Indicates whether requestWillBeSentExtraInfo and responseReceivedExtraInfo events will be
    /// or were emitted for this request.
    pub has_extra_info: bool,
    /// Frame identifier.
    #[serde(default)]
    pub frame_id: Option<super::page::FrameId>,
}

/// Event payload for `Network.webSocketClosed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketClosedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.webSocketCreated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketCreatedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// WebSocket request URL.
    pub url: String,
    /// Request initiator.
    #[serde(default)]
    pub initiator: Option<Initiator>,
}

/// Event payload for `Network.webSocketFrameError`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketFrameErrorEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// WebSocket error message.
    pub error_message: String,
}

/// Event payload for `Network.webSocketFrameReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketFrameReceivedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// WebSocket response data.
    pub response: WebSocketFrame,
}

/// Event payload for `Network.webSocketFrameSent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketFrameSentEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// WebSocket response data.
    pub response: WebSocketFrame,
}

/// Event payload for `Network.webSocketHandshakeResponseReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketHandshakeResponseReceivedEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// WebSocket response data.
    pub response: WebSocketResponse,
}

/// Event payload for `Network.webSocketWillSendHandshakeRequest`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketWillSendHandshakeRequestEvent {
    /// Request identifier.
    pub request_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// UTC Timestamp.
    pub wall_time: TimeSinceEpoch,
    /// WebSocket request data.
    pub request: WebSocketRequest,
}

/// Event payload for `Network.webTransportCreated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebTransportCreatedEvent {
    /// WebTransport identifier.
    pub transport_id: RequestId,
    /// WebTransport request URL.
    pub url: String,
    /// Timestamp.
    pub timestamp: MonotonicTime,
    /// Request initiator.
    #[serde(default)]
    pub initiator: Option<Initiator>,
}

/// Event payload for `Network.webTransportConnectionEstablished`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebTransportConnectionEstablishedEvent {
    /// WebTransport identifier.
    pub transport_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.webTransportClosed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebTransportClosedEvent {
    /// WebTransport identifier.
    pub transport_id: RequestId,
    /// Timestamp.
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directTCPSocketCreated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectTCPSocketCreatedEvent {
    pub identifier: RequestId,
    pub remote_addr: String,
    /// Unsigned int 16.
    pub remote_port: i64,
    pub options: DirectTCPSocketOptions,
    pub timestamp: MonotonicTime,
    #[serde(default)]
    pub initiator: Option<Initiator>,
}

/// Event payload for `Network.directTCPSocketOpened`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectTCPSocketOpenedEvent {
    pub identifier: RequestId,
    pub remote_addr: String,
    /// Expected to be unsigned integer.
    pub remote_port: i64,
    pub timestamp: MonotonicTime,
    #[serde(default)]
    pub local_addr: Option<String>,
    /// Expected to be unsigned integer.
    #[serde(default)]
    pub local_port: Option<i64>,
}

/// Event payload for `Network.directTCPSocketAborted`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectTCPSocketAbortedEvent {
    pub identifier: RequestId,
    pub error_message: String,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directTCPSocketClosed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectTCPSocketClosedEvent {
    pub identifier: RequestId,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directTCPSocketChunkSent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectTCPSocketChunkSentEvent {
    pub identifier: RequestId,
    pub data: String,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directTCPSocketChunkReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectTCPSocketChunkReceivedEvent {
    pub identifier: RequestId,
    pub data: String,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directUDPSocketJoinedMulticastGroup`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketJoinedMulticastGroupEvent {
    pub identifier: RequestId,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
}

/// Event payload for `Network.directUDPSocketLeftMulticastGroup`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketLeftMulticastGroupEvent {
    pub identifier: RequestId,
    #[serde(rename = "IPAddress")]
    pub ip_address: String,
}

/// Event payload for `Network.directUDPSocketCreated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketCreatedEvent {
    pub identifier: RequestId,
    pub options: DirectUDPSocketOptions,
    pub timestamp: MonotonicTime,
    #[serde(default)]
    pub initiator: Option<Initiator>,
}

/// Event payload for `Network.directUDPSocketOpened`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketOpenedEvent {
    pub identifier: RequestId,
    pub local_addr: String,
    /// Expected to be unsigned integer.
    pub local_port: i64,
    pub timestamp: MonotonicTime,
    #[serde(default)]
    pub remote_addr: Option<String>,
    /// Expected to be unsigned integer.
    #[serde(default)]
    pub remote_port: Option<i64>,
}

/// Event payload for `Network.directUDPSocketAborted`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketAbortedEvent {
    pub identifier: RequestId,
    pub error_message: String,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directUDPSocketClosed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketClosedEvent {
    pub identifier: RequestId,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directUDPSocketChunkSent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketChunkSentEvent {
    pub identifier: RequestId,
    pub message: DirectUDPMessage,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.directUDPSocketChunkReceived`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectUDPSocketChunkReceivedEvent {
    pub identifier: RequestId,
    pub message: DirectUDPMessage,
    pub timestamp: MonotonicTime,
}

/// Event payload for `Network.requestWillBeSentExtraInfo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestWillBeSentExtraInfoEvent {
    /// Request identifier. Used to match this information to an existing requestWillBeSent event.
    pub request_id: RequestId,
    /// A list of cookies potentially associated to the requested URL. This includes both cookies sent with
    /// the request and the ones not sent; the latter are distinguished by having blockedReasons field set.
    pub associated_cookies: Vec<AssociatedCookie>,
    /// Raw request headers as they will be sent over the wire.
    pub headers: Headers,
    /// Connection timing information for the request.
    pub connect_timing: ConnectTiming,
    /// The client security state set for the request.
    #[serde(default)]
    pub client_security_state: Option<ClientSecurityState>,
    /// Whether the site has partitioned cookies stored in a partition different than the current one.
    #[serde(default)]
    pub site_has_cookie_in_other_partition: Option<bool>,
    /// The network conditions id if this request was affected by network conditions configured via
    /// emulateNetworkConditionsByRule.
    #[serde(default)]
    pub applied_network_conditions_id: Option<String>,
}

/// Event payload for `Network.responseReceivedExtraInfo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseReceivedExtraInfoEvent {
    /// Request identifier. Used to match this information to another responseReceived event.
    pub request_id: RequestId,
    /// A list of cookies which were not stored from the response along with the corresponding
    /// reasons for blocking. The cookies here may not be valid due to syntax errors, which
    /// are represented by the invalid cookie line string instead of a proper cookie.
    pub blocked_cookies: Vec<BlockedSetCookieWithReason>,
    /// Raw response headers as they were received over the wire.
    /// Duplicate headers in the response are represented as a single key with their values
    /// concatentated using `\n` as the separator.
    /// See also `headersText` that contains verbatim text for HTTP/1.*.
    pub headers: Headers,
    /// The IP address space of the resource. The address space can only be determined once the transport
    /// established the connection, so we can't send it in `requestWillBeSentExtraInfo`.
    #[serde(rename = "resourceIPAddressSpace")]
    pub resource_ip_address_space: IPAddressSpace,
    /// The status code of the response. This is useful in cases the request failed and no responseReceived
    /// event is triggered, which is the case for, e.g., CORS errors. This is also the correct status code
    /// for cached requests, where the status in responseReceived is a 200 and this will be 304.
    pub status_code: i64,
    /// Raw response header text as it was received over the wire. The raw text may not always be
    /// available, such as in the case of HTTP/2 or QUIC.
    #[serde(default)]
    pub headers_text: Option<String>,
    /// The cookie partition key that will be used to store partitioned cookies set in this response.
    /// Only sent when partitioned cookies are enabled.
    #[serde(default)]
    pub cookie_partition_key: Option<CookiePartitionKey>,
    /// True if partitioned cookies are enabled, but the partition key is not serializable to string.
    #[serde(default)]
    pub cookie_partition_key_opaque: Option<bool>,
    /// A list of cookies which should have been blocked by 3PCD but are exempted and stored from
    /// the response with the corresponding reason.
    #[serde(default)]
    pub exempted_cookies: Option<Vec<ExemptedSetCookieWithReason>>,
}

/// Event payload for `Network.responseReceivedEarlyHints`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseReceivedEarlyHintsEvent {
    /// Request identifier. Used to match this information to another responseReceived event.
    pub request_id: RequestId,
    /// Raw response headers as they were received over the wire.
    /// Duplicate headers in the response are represented as a single key with their values
    /// concatentated using `\n` as the separator.
    /// See also `headersText` that contains verbatim text for HTTP/1.*.
    pub headers: Headers,
}

/// Event payload for `Network.trustTokenOperationDone`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustTokenOperationDoneEvent {
    /// Detailed success or error status of the operation.
    /// 'AlreadyExists' also signifies a successful operation, as the result
    /// of the operation already exists und thus, the operation was abort
    /// preemptively (e.g. a cache hit).
    pub status: String,
    #[serde(rename = "type")]
    pub r#type: TrustTokenOperationType,
    pub request_id: RequestId,
    /// Top level origin. The context in which the operation was attempted.
    #[serde(default)]
    pub top_level_origin: Option<String>,
    /// Origin of the issuer in case of a "Issuance" or "Redemption" operation.
    #[serde(default)]
    pub issuer_origin: Option<String>,
    /// The number of obtained Trust Tokens on a successful "Issuance" operation.
    #[serde(default)]
    pub issued_token_count: Option<i64>,
}

/// Event payload for `Network.reportingApiReportAdded`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingApiReportAddedEvent {
    pub report: ReportingApiReport,
}

/// Event payload for `Network.reportingApiReportUpdated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingApiReportUpdatedEvent {
    pub report: ReportingApiReport,
}

/// Event payload for `Network.reportingApiEndpointsChangedForOrigin`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportingApiEndpointsChangedForOriginEvent {
    /// Origin of the document(s) which configured the endpoints.
    pub origin: String,
    pub endpoints: Vec<ReportingApiEndpoint>,
}

// ── Methods ──
//
// These are the typed method signatures for Network.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Sets a list of content encodings that will be accepted. Empty list means no encoding is accepted.
// async fn network_set_accepted_encodings(&self, params: SetAcceptedEncodingsParams) -> Result<()>
// CDP: "Network.setAcceptedEncodings"

// Clears accepted encodings set by setAcceptedEncodings
// async fn network_clear_accepted_encodings_override(&self) -> Result<()>
// CDP: "Network.clearAcceptedEncodingsOverride"

// Tells whether clearing browser cache is supported.
// async fn network_can_clear_browser_cache(&self) -> Result<CanClearBrowserCacheReturns>
// CDP: "Network.canClearBrowserCache"

// Tells whether clearing browser cookies is supported.
// async fn network_can_clear_browser_cookies(&self) -> Result<CanClearBrowserCookiesReturns>
// CDP: "Network.canClearBrowserCookies"

// Tells whether emulation of network conditions is supported.
// async fn network_can_emulate_network_conditions(&self) -> Result<CanEmulateNetworkConditionsReturns>
// CDP: "Network.canEmulateNetworkConditions"

// Clears browser cache.
// async fn network_clear_browser_cache(&self) -> Result<()>
// CDP: "Network.clearBrowserCache"

// Clears browser cookies.
// async fn network_clear_browser_cookies(&self) -> Result<()>
// CDP: "Network.clearBrowserCookies"

// Response to Network.requestIntercepted which either modifies the request to continue with any
// modifications, or blocks it, or completes it with the provided response bytes. If a network
// fetch occurs as a result which encounters a redirect an additional Network.requestIntercepted
// event will be sent with the same InterceptionId.
// Deprecated, use Fetch.continueRequest, Fetch.fulfillRequest and Fetch.failRequest instead.
// async fn network_continue_intercepted_request(&self, params: ContinueInterceptedRequestParams) -> Result<()>
// CDP: "Network.continueInterceptedRequest"

// Deletes browser cookies with matching name and url or domain/path/partitionKey pair.
// async fn network_delete_cookies(&self, params: DeleteCookiesParams) -> Result<()>
// CDP: "Network.deleteCookies"

// Disables network tracking, prevents network events from being sent to the client.
// async fn network_disable(&self) -> Result<()>
// CDP: "Network.disable"

// Activates emulation of network conditions. This command is deprecated in favor of the emulateNetworkConditionsByRule
// and overrideNetworkState commands, which can be used together to the same effect.
// async fn network_emulate_network_conditions(&self, params: EmulateNetworkConditionsParams) -> Result<()>
// CDP: "Network.emulateNetworkConditions"

// Activates emulation of network conditions for individual requests using URL match patterns. Unlike the deprecated
// Network.emulateNetworkConditions this method does not affect `navigator` state. Use Network.overrideNetworkState to
// explicitly modify `navigator` behavior.
// async fn network_emulate_network_conditions_by_rule(&self, params: EmulateNetworkConditionsByRuleParams) -> Result<EmulateNetworkConditionsByRuleReturns>
// CDP: "Network.emulateNetworkConditionsByRule"

// Override the state of navigator.onLine and navigator.connection.
// async fn network_override_network_state(&self, params: OverrideNetworkStateParams) -> Result<()>
// CDP: "Network.overrideNetworkState"

// Enables network tracking, network events will now be delivered to the client.
// async fn network_enable(&self, params: EnableParams) -> Result<()>
// CDP: "Network.enable"

// Returns all browser cookies. Depending on the backend support, will return detailed cookie
// information in the `cookies` field.
// Deprecated. Use Storage.getCookies instead.
// async fn network_get_all_cookies(&self) -> Result<GetAllCookiesReturns>
// CDP: "Network.getAllCookies"

// Returns the DER-encoded certificate.
// async fn network_get_certificate(&self, params: GetCertificateParams) -> Result<GetCertificateReturns>
// CDP: "Network.getCertificate"

// Returns all browser cookies for the current URL. Depending on the backend support, will return
// detailed cookie information in the `cookies` field.
// async fn network_get_cookies(&self, params: GetCookiesParams) -> Result<GetCookiesReturns>
// CDP: "Network.getCookies"

// Returns content served for the given request.
// async fn network_get_response_body(&self, params: GetResponseBodyParams) -> Result<GetResponseBodyReturns>
// CDP: "Network.getResponseBody"

// Returns post data sent with the request. Returns an error when no data was sent with the request.
// async fn network_get_request_post_data(&self, params: GetRequestPostDataParams) -> Result<GetRequestPostDataReturns>
// CDP: "Network.getRequestPostData"

// Returns content served for the given currently intercepted request.
// async fn network_get_response_body_for_interception(&self, params: GetResponseBodyForInterceptionParams) -> Result<GetResponseBodyForInterceptionReturns>
// CDP: "Network.getResponseBodyForInterception"

// Returns a handle to the stream representing the response body. Note that after this command,
// the intercepted request can't be continued as is -- you either need to cancel it or to provide
// the response body. The stream only supports sequential read, IO.read will fail if the position
// is specified.
// async fn network_take_response_body_for_interception_as_stream(&self, params: TakeResponseBodyForInterceptionAsStreamParams) -> Result<TakeResponseBodyForInterceptionAsStreamReturns>
// CDP: "Network.takeResponseBodyForInterceptionAsStream"

// This method sends a new XMLHttpRequest which is identical to the original one. The following
// parameters should be identical: method, url, async, request body, extra headers, withCredentials
// attribute, user, password.
// async fn network_replay_xhr(&self, params: ReplayXHRParams) -> Result<()>
// CDP: "Network.replayXHR"

// Searches for given string in response content.
// async fn network_search_in_response_body(&self, params: SearchInResponseBodyParams) -> Result<SearchInResponseBodyReturns>
// CDP: "Network.searchInResponseBody"

// Blocks URLs from loading.
// async fn network_set_blocked_ur_ls(&self, params: SetBlockedURLsParams) -> Result<()>
// CDP: "Network.setBlockedURLs"

// Toggles ignoring of service worker for each request.
// async fn network_set_bypass_service_worker(&self, params: SetBypassServiceWorkerParams) -> Result<()>
// CDP: "Network.setBypassServiceWorker"

// Toggles ignoring cache for each request. If `true`, cache will not be used.
// async fn network_set_cache_disabled(&self, params: SetCacheDisabledParams) -> Result<()>
// CDP: "Network.setCacheDisabled"

// Sets a cookie with the given cookie data; may overwrite equivalent cookies if they exist.
// async fn network_set_cookie(&self, params: SetCookieParams) -> Result<SetCookieReturns>
// CDP: "Network.setCookie"

// Sets given cookies.
// async fn network_set_cookies(&self, params: SetCookiesParams) -> Result<()>
// CDP: "Network.setCookies"

// Specifies whether to always send extra HTTP headers with the requests from this page.
// async fn network_set_extra_http_headers(&self, params: SetExtraHTTPHeadersParams) -> Result<()>
// CDP: "Network.setExtraHTTPHeaders"

// Specifies whether to attach a page script stack id in requests
// async fn network_set_attach_debug_stack(&self, params: SetAttachDebugStackParams) -> Result<()>
// CDP: "Network.setAttachDebugStack"

// Sets the requests to intercept that match the provided patterns and optionally resource types.
// Deprecated, please use Fetch.enable instead.
// async fn network_set_request_interception(&self, params: SetRequestInterceptionParams) -> Result<()>
// CDP: "Network.setRequestInterception"

// Allows overriding user agent with the given string.
// async fn network_set_user_agent_override(&self, params: SetUserAgentOverrideParams) -> Result<()>
// CDP: "Network.setUserAgentOverride"

// Enables streaming of the response for the given requestId.
// If enabled, the dataReceived event contains the data that was received during streaming.
// async fn network_stream_resource_content(&self, params: StreamResourceContentParams) -> Result<StreamResourceContentReturns>
// CDP: "Network.streamResourceContent"

// Returns information about the COEP/COOP isolation status.
// async fn network_get_security_isolation_status(&self, params: GetSecurityIsolationStatusParams) -> Result<GetSecurityIsolationStatusReturns>
// CDP: "Network.getSecurityIsolationStatus"

// Enables tracking for the Reporting API, events generated by the Reporting API will now be delivered to the client.
// Enabling triggers 'reportingApiReportAdded' for all existing reports.
// async fn network_enable_reporting_api(&self, params: EnableReportingApiParams) -> Result<()>
// CDP: "Network.enableReportingApi"

// Fetches the resource and returns the content.
// async fn network_load_network_resource(&self, params: LoadNetworkResourceParams) -> Result<LoadNetworkResourceReturns>
// CDP: "Network.loadNetworkResource"

// Sets Controls for third-party cookie access
// Page reload is required before the new cookie behavior will be observed
// async fn network_set_cookie_controls(&self, params: SetCookieControlsParams) -> Result<()>
// CDP: "Network.setCookieControls"
