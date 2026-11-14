//! Typed, bounded conversion of the opt-in Rust Playwright Server Page stream.
//! No browser observation or event ordering is implemented at this boundary.

use napi_derive::napi;
use pwright_playwright_server_session::PageNotice;

#[napi(object)]
pub struct NativeRemoteObjectId {
    pub guid: String,
    /// Decimal string so Node never rounds a long-lived GUID generation.
    pub generation: String,
}

impl From<pwright_playwright_server_page_events::ObjectId> for NativeRemoteObjectId {
    fn from(value: pwright_playwright_server_page_events::ObjectId) -> Self {
        Self {
            guid: value.guid,
            generation: value.generation.to_string(),
        }
    }
}

#[napi(object)]
pub struct NativeRemoteHeader {
    pub name: String,
    pub value: String,
}

impl From<pwright_playwright_server_page_events::Header> for NativeRemoteHeader {
    fn from(value: pwright_playwright_server_page_events::Header) -> Self {
        Self {
            name: value.name,
            value: value.value,
        }
    }
}

#[napi(object)]
pub struct NativeRemoteRequestSnapshot {
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<NativeRemoteHeader>,
    pub is_navigation_request: bool,
}

impl From<pwright_playwright_server_page_events::RequestSnapshot> for NativeRemoteRequestSnapshot {
    fn from(value: pwright_playwright_server_page_events::RequestSnapshot) -> Self {
        Self {
            url: value.url,
            method: value.method,
            resource_type: value.resource_type,
            headers: value.headers.into_iter().map(Into::into).collect(),
            is_navigation_request: value.is_navigation_request,
        }
    }
}

#[napi(object)]
pub struct NativeRemoteResponseSnapshot {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<NativeRemoteHeader>,
    pub from_service_worker: bool,
    pub request: NativeRemoteObjectId,
}

impl From<pwright_playwright_server_page_events::ResponseSnapshot>
    for NativeRemoteResponseSnapshot
{
    fn from(value: pwright_playwright_server_page_events::ResponseSnapshot) -> Self {
        Self {
            url: value.url,
            status: value.status,
            status_text: value.status_text,
            headers: value.headers.into_iter().map(Into::into).collect(),
            from_service_worker: value.from_service_worker,
            request: value.request.into(),
        }
    }
}

#[napi(object)]
pub struct NativeRemotePageNotice {
    pub kind: String,
    pub page: Option<NativeRemoteObjectId>,
    pub object: Option<NativeRemoteObjectId>,
    pub request: Option<NativeRemoteRequestSnapshot>,
    pub response: Option<NativeRemoteResponseSnapshot>,
    pub request_snapshot: Option<NativeRemoteRequestSnapshot>,
    pub url: Option<String>,
    pub suggested_filename: Option<String>,
}

impl NativeRemotePageNotice {
    fn terminal(kind: &str, object: Option<NativeRemoteObjectId>) -> Self {
        Self {
            kind: kind.to_owned(),
            page: None,
            object,
            request: None,
            response: None,
            request_snapshot: None,
            url: None,
            suggested_filename: None,
        }
    }
}

impl From<PageNotice> for NativeRemotePageNotice {
    fn from(value: PageNotice) -> Self {
        match value {
            PageNotice::Request {
                page,
                request,
                snapshot,
            } => Self {
                kind: "request".to_owned(),
                page: Some(page.into()),
                object: Some(request.into()),
                request: Some(snapshot.into()),
                response: None,
                request_snapshot: None,
                url: None,
                suggested_filename: None,
            },
            PageNotice::Response {
                page,
                response,
                snapshot,
                request_snapshot,
            } => Self {
                kind: "response".to_owned(),
                page: Some(page.into()),
                object: Some(response.into()),
                request: None,
                response: Some(snapshot.into()),
                request_snapshot: Some(request_snapshot.into()),
                url: None,
                suggested_filename: None,
            },
            PageNotice::Download {
                page,
                artifact,
                url,
                suggested_filename,
            } => Self {
                kind: "download".to_owned(),
                page: Some(page.into()),
                object: Some(artifact.into()),
                request: None,
                response: None,
                request_snapshot: None,
                url: Some(url),
                suggested_filename: Some(suggested_filename),
            },
            PageNotice::PageClosed(id) => Self::terminal("pageClosed", Some(id.into())),
            PageNotice::PageCrashed(id) => Self::terminal("pageCrashed", Some(id.into())),
            PageNotice::ContextClosed(id) => Self::terminal("contextClosed", Some(id.into())),
            PageNotice::BrowserClosed(id) => Self::terminal("browserClosed", Some(id.into())),
            PageNotice::Disconnected => Self::terminal("disconnected", None),
        }
    }
}
