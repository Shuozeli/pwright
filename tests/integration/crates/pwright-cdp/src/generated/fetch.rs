//! CDP `Fetch` domain — generated from protocol JSON.
//!
//! A domain for letting clients substitute browser's network layer with client code.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unique request identifier.
/// Note that this does not identify individual HTTP requests that are part of
/// a network request.
pub type RequestId = String;

/// Stages of the request to handle. Request will intercept before the request is
/// sent. Response will intercept after the response is received (but before response
/// body is received).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RequestStage {
    Request,
    Response,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RequestPattern {
    /// Wildcards (`'*'` -> zero or more, `'?'` -> exactly one) are allowed. Escape character is
    /// backslash. Omitting is equivalent to `"*"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    /// If set, only requests for matching resource types will be intercepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<super::network::ResourceType>,
    /// Stage at which to begin intercepting requests. Default is Request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_stage: Option<RequestStage>,
}

/// Response HTTP header entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderEntry {
    pub name: String,
    pub value: String,
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

/// Parameters for `Fetch.enable`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// If specified, only requests matching any of these patterns will produce
    /// fetchRequested event and will be paused until clients response. If not set,
    /// all requests will be affected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patterns: Option<Vec<RequestPattern>>,
    /// If true, authRequired events will be issued and requests will be paused
    /// expecting a call to continueWithAuth.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle_auth_requests: Option<bool>,
}

/// Parameters for `Fetch.failRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailRequestParams {
    /// An id the client received in requestPaused event.
    pub request_id: RequestId,
    /// Causes the request to fail with the given reason.
    pub error_reason: super::network::ErrorReason,
}

/// Parameters for `Fetch.fulfillRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FulfillRequestParams {
    /// An id the client received in requestPaused event.
    pub request_id: RequestId,
    /// An HTTP response code.
    pub response_code: i64,
    /// Response headers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<Vec<HeaderEntry>>,
    /// Alternative way of specifying response headers as a \0-separated
    /// series of name: value pairs. Prefer the above method unless you
    /// need to represent some non-UTF8 values that can't be transmitted
    /// over the protocol as text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_response_headers: Option<String>,
    /// A response body. If absent, original response body will be used if
    /// the request is intercepted at the response stage and empty body
    /// will be used if the request is intercepted at the request stage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// A textual representation of responseCode.
    /// If absent, a standard phrase matching responseCode is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_phrase: Option<String>,
}

/// Parameters for `Fetch.continueRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueRequestParams {
    /// An id the client received in requestPaused event.
    pub request_id: RequestId,
    /// If set, the request url will be modified in a way that's not observable by page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// If set, the request method is overridden.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    /// If set, overrides the post data in the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_data: Option<String>,
    /// If set, overrides the request headers. Note that the overrides do not
    /// extend to subsequent redirect hops, if a redirect happens. Another override
    /// may be applied to a different request produced by a redirect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<Vec<HeaderEntry>>,
    /// If set, overrides response interception behavior for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intercept_response: Option<bool>,
}

/// Parameters for `Fetch.continueWithAuth`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueWithAuthParams {
    /// An id the client received in authRequired event.
    pub request_id: RequestId,
    /// Response to  with an authChallenge.
    pub auth_challenge_response: AuthChallengeResponse,
}

/// Parameters for `Fetch.continueResponse`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueResponseParams {
    /// An id the client received in requestPaused event.
    pub request_id: RequestId,
    /// An HTTP response code. If absent, original response code will be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_code: Option<i64>,
    /// A textual representation of responseCode.
    /// If absent, a standard phrase matching responseCode is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_phrase: Option<String>,
    /// Response headers. If absent, original response headers will be used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_headers: Option<Vec<HeaderEntry>>,
    /// Alternative way of specifying response headers as a \0-separated
    /// series of name: value pairs. Prefer the above method unless you
    /// need to represent some non-UTF8 values that can't be transmitted
    /// over the protocol as text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_response_headers: Option<String>,
}

/// Parameters for `Fetch.getResponseBody`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseBodyParams {
    /// Identifier for the intercepted request to get body for.
    pub request_id: RequestId,
}

/// Return type for `Fetch.getResponseBody`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResponseBodyReturns {
    /// Response body.
    pub body: String,
    /// True, if content was sent as base64.
    pub base64_encoded: bool,
}

/// Parameters for `Fetch.takeResponseBodyAsStream`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeResponseBodyAsStreamParams {
    pub request_id: RequestId,
}

/// Return type for `Fetch.takeResponseBodyAsStream`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TakeResponseBodyAsStreamReturns {
    pub stream: Value,
}

/// Event payload for `Fetch.requestPaused`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestPausedEvent {
    /// Each request the page makes will have a unique id.
    pub request_id: RequestId,
    /// The details of the request.
    pub request: super::network::Request,
    /// The id of the frame that initiated the request.
    pub frame_id: super::page::FrameId,
    /// How the requested resource will be used.
    pub resource_type: super::network::ResourceType,
    /// Response error if intercepted at response stage.
    #[serde(default)]
    pub response_error_reason: Option<super::network::ErrorReason>,
    /// Response code if intercepted at response stage.
    #[serde(default)]
    pub response_status_code: Option<i64>,
    /// Response status text if intercepted at response stage.
    #[serde(default)]
    pub response_status_text: Option<String>,
    /// Response headers if intercepted at the response stage.
    #[serde(default)]
    pub response_headers: Option<Vec<HeaderEntry>>,
    /// If the intercepted request had a corresponding Network.requestWillBeSent event fired for it,
    /// then this networkId will be the same as the requestId present in the requestWillBeSent event.
    #[serde(default)]
    pub network_id: Option<super::network::RequestId>,
    /// If the request is due to a redirect response from the server, the id of the request that
    /// has caused the redirect.
    #[serde(default)]
    pub redirected_request_id: Option<RequestId>,
}

/// Event payload for `Fetch.authRequired`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRequiredEvent {
    /// Each request the page makes will have a unique id.
    pub request_id: RequestId,
    /// The details of the request.
    pub request: super::network::Request,
    /// The id of the frame that initiated the request.
    pub frame_id: super::page::FrameId,
    /// How the requested resource will be used.
    pub resource_type: super::network::ResourceType,
    /// Details of the Authorization Challenge encountered.
    /// If this is set, client should respond with continueRequest that
    /// contains AuthChallengeResponse.
    pub auth_challenge: AuthChallenge,
}

// ── Methods ──
//
// These are the typed method signatures for Fetch.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Disables the fetch domain.
// async fn fetch_disable(&self) -> Result<()>
// CDP: "Fetch.disable"

// Enables issuing of requestPaused events. A request will be paused until client
// calls one of failRequest, fulfillRequest or continueRequest/continueWithAuth.
// async fn fetch_enable(&self, params: EnableParams) -> Result<()>
// CDP: "Fetch.enable"

// Causes the request to fail with specified reason.
// async fn fetch_fail_request(&self, params: FailRequestParams) -> Result<()>
// CDP: "Fetch.failRequest"

// Provides response to the request.
// async fn fetch_fulfill_request(&self, params: FulfillRequestParams) -> Result<()>
// CDP: "Fetch.fulfillRequest"

// Continues the request, optionally modifying some of its parameters.
// async fn fetch_continue_request(&self, params: ContinueRequestParams) -> Result<()>
// CDP: "Fetch.continueRequest"

// Continues a request supplying authChallengeResponse following authRequired event.
// async fn fetch_continue_with_auth(&self, params: ContinueWithAuthParams) -> Result<()>
// CDP: "Fetch.continueWithAuth"

// Continues loading of the paused response, optionally modifying the
// response headers. If either responseCode or headers are modified, all of them
// must be present.
// async fn fetch_continue_response(&self, params: ContinueResponseParams) -> Result<()>
// CDP: "Fetch.continueResponse"

// Causes the body of the response to be received from the server and
// returned as a single string. May only be issued for a request that
// is paused in the Response stage and is mutually exclusive with
// takeResponseBodyForInterceptionAsStream. Calling other methods that
// affect the request or disabling fetch domain before body is received
// results in an undefined behavior.
// Note that the response body is not available for redirects. Requests
// paused in the _redirect received_ state may be differentiated by
// `responseCode` and presence of `location` response header, see
// comments to `requestPaused` for details.
// async fn fetch_get_response_body(&self, params: GetResponseBodyParams) -> Result<GetResponseBodyReturns>
// CDP: "Fetch.getResponseBody"

// Returns a handle to the stream representing the response body.
// The request must be paused in the HeadersReceived stage.
// Note that after this command the request can't be continued
// as is -- client either needs to cancel it or to provide the
// response body.
// The stream only supports sequential read, IO.read will fail if the position
// is specified.
// This method is mutually exclusive with getResponseBody.
// Calling other methods that affect the request or disabling fetch
// domain before body is received results in an undefined behavior.
// async fn fetch_take_response_body_as_stream(&self, params: TakeResponseBodyAsStreamParams) -> Result<TakeResponseBodyAsStreamReturns>
// CDP: "Fetch.takeResponseBodyAsStream"

