//! A default-off, transport-free projection of validated Playwright Server ingress.
//!
//! Feed one wire-ordered `Ingress` stream from a single supervisor. This leaf
//! neither owns that stream nor sends `Page.updateSubscription`; callers must
//! install request/response subscriptions before expecting those notices. The leaf
//! retains bounded metadata so a notice remains usable after its wire object
//! is disposed. A projection error is terminal for this projector and must be
//! treated as a session failure by the caller.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use pwright_playwright_server_client_core::Ingress;
use serde_json::{Map, Value};

pub const MAX_PROJECTED_OBJECTS: usize = 4_096;
pub const MAX_PROJECTED_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_QUEUED_NOTICES: usize = 128;
pub const MAX_QUEUED_NOTICE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_URL_BYTES: usize = 64 * 1024;
pub const MAX_FILENAME_BYTES: usize = 4 * 1024;
pub const MAX_HEADERS: usize = 128;
pub const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_GUID_BYTES: usize = 512;
const OBJECT_OVERHEAD: usize = 128;
const NOTICE_OVERHEAD: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectId {
    pub guid: String,
    /// Monotonic within this projection, including across disposal and reuse.
    pub generation: u64,
}

#[derive(Clone, Eq, PartialEq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Eq, PartialEq)]
pub struct RequestSnapshot {
    pub url: String,
    pub method: String,
    pub resource_type: String,
    pub headers: Vec<Header>,
    pub is_navigation_request: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub struct ResponseSnapshot {
    pub url: String,
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<Header>,
    pub from_service_worker: bool,
    pub request: ObjectId,
}

/// No server absolute path, response body, request body or Artifact stream is
/// copied into a notice. Those methods require distinct capabilities.
#[derive(Clone)]
pub enum Notice {
    Request {
        page: ObjectId,
        request: ObjectId,
        snapshot: RequestSnapshot,
    },
    Response {
        page: ObjectId,
        response: ObjectId,
        snapshot: ResponseSnapshot,
        request_snapshot: RequestSnapshot,
    },
    Download {
        page: ObjectId,
        artifact: ObjectId,
        url: String,
        suggested_filename: String,
    },
    PageClosed(ObjectId),
    PageCrashed(ObjectId),
    ContextClosed(ObjectId),
    BrowserClosed(ObjectId),
    /// Emitted only by `finish`, after all queued wire notices and child closes.
    Disconnected,
}

/// The bounded close-only policy never publishes network, download, or crash
/// payloads. Object identities remain stable after wire disposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CloseNotice {
    PageClosed(ObjectId),
    ContextClosed(ObjectId),
    BrowserClosed(ObjectId),
    Disconnected,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ProjectionMode {
    PageEvents,
    CloseOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionError {
    Invalid(&'static str),
    ObjectLimit,
    MetadataLimit,
    NoticeLimit,
    Finished,
    Failed,
}

enum Kind {
    Root,
    BrowserType,
    Browser,
    Context,
    Page,
    Frame,
    Request(RequestSnapshot),
    Response(ResponseSnapshot),
    Artifact,
    Other,
}

impl Kind {
    fn from_type(object_type: &str) -> Self {
        match object_type {
            "BrowserType" => Self::BrowserType,
            "Browser" => Self::Browser,
            "BrowserContext" => Self::Context,
            "Page" => Self::Page,
            "Frame" => Self::Frame,
            "Artifact" => Self::Artifact,
            _ => Self::Other,
        }
    }

    fn metadata_bytes(&self) -> usize {
        match self {
            Self::Request(value) => request_bytes(value),
            Self::Response(value) => response_bytes(value),
            _ => 0,
        }
    }
}

struct Entry {
    parent: Option<String>,
    kind: Kind,
    generation: u64,
    closed: bool,
    retained_bytes: usize,
}

impl Entry {
    fn id(&self, guid: &str) -> ObjectId {
        ObjectId {
            guid: guid.to_owned(),
            generation: self.generation,
        }
    }
}

struct QueuedNotice {
    notice: Notice,
    bytes: usize,
}

/// A single-consumer, bounded notice queue and the minimum object metadata
/// needed to identify Page-owned protocol events at their wire position.
pub struct PageEventProjection {
    mode: ProjectionMode,
    objects: BTreeMap<String, Entry>,
    next_generation: u64,
    retained_bytes: usize,
    notices: VecDeque<QueuedNotice>,
    notice_bytes: usize,
    failed: bool,
    finished: bool,
}

impl Default for PageEventProjection {
    fn default() -> Self {
        Self::new()
    }
}

impl PageEventProjection {
    pub fn new() -> Self {
        Self::with_mode(ProjectionMode::PageEvents)
    }

    /// Track only BrowserType, Browser, Context, and Page topology with ordered
    /// close notices. Other objects advance the generation without retention.
    pub fn new_close_only() -> Self {
        Self::with_mode(ProjectionMode::CloseOnly)
    }

    fn with_mode(mode: ProjectionMode) -> Self {
        let root = Entry {
            parent: None,
            kind: Kind::Root,
            generation: 0,
            closed: false,
            retained_bytes: OBJECT_OVERHEAD,
        };
        Self {
            mode,
            objects: BTreeMap::from([(String::new(), root)]),
            next_generation: 1,
            retained_bytes: OBJECT_OVERHEAD,
            notices: VecDeque::new(),
            notice_bytes: 0,
            failed: false,
            finished: false,
        }
    }

    pub fn accept(&mut self, ingress: &Ingress) -> Result<(), ProjectionError> {
        if self.failed {
            return Err(ProjectionError::Failed);
        }
        if self.finished {
            return Err(ProjectionError::Finished);
        }
        let result = self.accept_inner(ingress);
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    pub fn finish(&mut self) -> Result<(), ProjectionError> {
        if self.failed {
            return Err(ProjectionError::Failed);
        }
        if self.finished {
            return Ok(());
        }
        let mut notices = self.closes_for_all();
        notices.push(Notice::Disconnected);
        let result = self.publish(&notices);
        if result.is_err() {
            self.failed = true;
            return result;
        }
        self.mark_closed(&notices);
        self.finished = true;
        Ok(())
    }

    pub fn next_notice(&mut self) -> Option<Notice> {
        self.notices.pop_front().map(|queued| {
            self.notice_bytes -= queued.bytes;
            queued.notice
        })
    }

    /// Drain a close-only projection without exposing the broader Page notice
    /// type. A full Page-event projection has no close-only stream.
    pub fn next_close_notice(&mut self) -> Result<Option<CloseNotice>, ProjectionError> {
        if self.mode != ProjectionMode::CloseOnly {
            return Err(ProjectionError::Invalid("not a close-only projection"));
        }
        let notice = self.next_notice().map(|notice| match notice {
            Notice::PageClosed(id) => CloseNotice::PageClosed(id),
            Notice::ContextClosed(id) => CloseNotice::ContextClosed(id),
            Notice::BrowserClosed(id) => CloseNotice::BrowserClosed(id),
            Notice::Disconnected => CloseNotice::Disconnected,
            Notice::Request { .. }
            | Notice::Response { .. }
            | Notice::Download { .. }
            | Notice::PageCrashed(_) => unreachable!("close-only projection published payload"),
        });
        Ok(notice)
    }

    pub fn queued_notice_count(&self) -> usize {
        self.notices.len()
    }

    pub fn retained_object_bytes(&self) -> usize {
        self.retained_bytes
    }

    fn accept_inner(&mut self, ingress: &Ingress) -> Result<(), ProjectionError> {
        match ingress {
            Ingress::Created {
                parent,
                guid,
                object_type,
                initializer,
            } => self.create(parent, guid, object_type, initializer),
            Ingress::Adopted { parent, guid } => self.adopt(parent, guid),
            Ingress::Disposed { guids, .. } => self.dispose(guids),
            Ingress::Event {
                guid,
                method,
                params,
            } => self.event(guid, method, params),
            Ingress::Reply { .. } => Ok(()),
        }
    }

    fn create(
        &mut self,
        parent: &str,
        guid: &str,
        object_type: &str,
        initializer: &Value,
    ) -> Result<(), ProjectionError> {
        valid_guid(guid)?;
        let mut kind = Kind::from_type(object_type);
        if self.mode == ProjectionMode::CloseOnly
            && !matches!(
                &kind,
                Kind::BrowserType | Kind::Browser | Kind::Context | Kind::Page
            )
        {
            self.next_generation = self
                .next_generation
                .checked_add(1)
                .ok_or(ProjectionError::ObjectLimit)?;
            return Ok(());
        }
        if self.objects.contains_key(guid) || !self.objects.contains_key(parent) {
            return Err(ProjectionError::Invalid("create GUID or parent"));
        }
        if self.objects.len() >= MAX_PROJECTED_OBJECTS {
            return Err(ProjectionError::ObjectLimit);
        }
        if self.mode == ProjectionMode::PageEvents && object_type == "Request" {
            kind = Kind::Request(parse_request(initializer)?);
        } else if self.mode == ProjectionMode::PageEvents && object_type == "Response" {
            kind = Kind::Response(self.parse_response(parent, initializer)?);
        }
        self.validate_parent(&kind, parent)?;
        let retained_bytes = OBJECT_OVERHEAD
            .checked_add(parent.len())
            .and_then(|value| value.checked_add(guid.len()))
            .and_then(|value| value.checked_add(kind.metadata_bytes()))
            .ok_or(ProjectionError::MetadataLimit)?;
        let total = self
            .retained_bytes
            .checked_add(retained_bytes)
            .filter(|value| *value <= MAX_PROJECTED_BYTES)
            .ok_or(ProjectionError::MetadataLimit)?;
        let generation = self.next_generation;
        self.next_generation = generation
            .checked_add(1)
            .ok_or(ProjectionError::ObjectLimit)?;
        self.objects.insert(
            guid.to_owned(),
            Entry {
                parent: Some(parent.to_owned()),
                kind,
                generation,
                closed: false,
                retained_bytes,
            },
        );
        self.retained_bytes = total;
        Ok(())
    }

    fn parse_response(
        &self,
        parent: &str,
        initializer: &Value,
    ) -> Result<ResponseSnapshot, ProjectionError> {
        let source = object(initializer)?;
        let request_guid = channel_guid(source, "request")?;
        if request_guid != parent {
            return Err(ProjectionError::Invalid("Response request parent"));
        }
        let request = self.entry_as(request_guid, "Request")?;
        let status = source
            .get("status")
            .and_then(Value::as_u64)
            .and_then(|value| u16::try_from(value).ok())
            .filter(|value| *value <= 999)
            .ok_or(ProjectionError::Invalid("Response status"))?;
        Ok(ResponseSnapshot {
            url: field_string(source, "url", MAX_URL_BYTES)?,
            status,
            status_text: field_string(source, "statusText", 4 * 1024)?,
            headers: parse_headers(source)?,
            from_service_worker: source
                .get("fromServiceWorker")
                .and_then(Value::as_bool)
                .ok_or(ProjectionError::Invalid("Response service-worker flag"))?,
            request: request.id(request_guid),
        })
    }

    fn validate_parent(&self, kind: &Kind, parent: &str) -> Result<(), ProjectionError> {
        let parent_entry = self
            .objects
            .get(parent)
            .ok_or(ProjectionError::Invalid("unknown parent"))?;
        if parent_entry.closed {
            return Err(ProjectionError::Invalid("closed parent"));
        }
        let parent_kind = &parent_entry.kind;
        let valid = match kind {
            Kind::BrowserType => matches!(parent_kind, Kind::Root),
            Kind::Browser => matches!(parent_kind, Kind::BrowserType),
            Kind::Context => matches!(parent_kind, Kind::Browser),
            Kind::Page => matches!(parent_kind, Kind::Context),
            Kind::Frame => matches!(parent_kind, Kind::Context | Kind::Page),
            Kind::Request(_) => matches!(parent_kind, Kind::Context | Kind::Page | Kind::Frame),
            Kind::Response(_) => matches!(parent_kind, Kind::Request(_)),
            _ => true,
        };
        if valid {
            Ok(())
        } else {
            Err(ProjectionError::Invalid("object parent type"))
        }
    }

    fn adopt(&mut self, parent: &str, guid: &str) -> Result<(), ProjectionError> {
        valid_guid(guid)?;
        if self.mode == ProjectionMode::CloseOnly && !self.objects.contains_key(guid) {
            return Ok(());
        }
        let entry = self
            .objects
            .get(guid)
            .ok_or(ProjectionError::Invalid("adopted GUID"))?;
        if matches!(entry.kind, Kind::Root) {
            return Err(ProjectionError::Invalid("adopt Root"));
        }
        self.validate_parent(&entry.kind, parent)?;
        if self.has_ancestor(parent, guid)? {
            return Err(ProjectionError::Invalid("adoption cycle"));
        }
        let old_parent = entry
            .parent
            .as_ref()
            .ok_or(ProjectionError::Invalid("adopt Root"))?;
        let new_bytes = entry
            .retained_bytes
            .checked_sub(old_parent.len())
            .and_then(|value| value.checked_add(parent.len()))
            .ok_or(ProjectionError::MetadataLimit)?;
        let total = self
            .retained_bytes
            .checked_sub(entry.retained_bytes)
            .and_then(|value| value.checked_add(new_bytes))
            .filter(|value| *value <= MAX_PROJECTED_BYTES)
            .ok_or(ProjectionError::MetadataLimit)?;
        let entry = self.objects.get_mut(guid).expect("checked GUID");
        entry.parent = Some(parent.to_owned());
        entry.retained_bytes = new_bytes;
        self.retained_bytes = total;
        Ok(())
    }

    fn dispose(&mut self, guids: &[String]) -> Result<(), ProjectionError> {
        let mut removed = BTreeSet::new();
        for guid in guids {
            valid_guid(guid)?;
            if self.mode == ProjectionMode::CloseOnly && !self.objects.contains_key(guid) {
                continue;
            }
            if !removed.insert(guid.as_str()) || !self.objects.contains_key(guid) {
                return Err(ProjectionError::Invalid("disposed GUID"));
            }
        }
        let notices = self.closes_for_guids(&removed);
        self.publish(&notices)?;
        for guid in guids {
            if let Some(entry) = self.objects.remove(guid) {
                self.retained_bytes -= entry.retained_bytes;
            }
        }
        Ok(())
    }

    fn event(&mut self, guid: &str, method: &str, params: &Value) -> Result<(), ProjectionError> {
        if self.mode == ProjectionMode::CloseOnly && method != "close" {
            return Ok(());
        }
        let entry = self.objects.get(guid);
        let Some(entry) = entry else {
            return if self.mode == ProjectionMode::CloseOnly {
                Ok(())
            } else {
                Err(ProjectionError::Invalid("event target GUID"))
            };
        };
        let notices = match (&entry.kind, method) {
            (Kind::Context, "request") => self.request_notice(guid, params)?,
            (Kind::Context, "response") => self.response_notice(guid, params)?,
            (Kind::Page, "download") => self.download_notice(guid, params)?,
            (Kind::Page, "close") | (Kind::Context, "close") | (Kind::Browser, "close") => {
                if !params.is_object() {
                    return Err(ProjectionError::Invalid("close event params"));
                }
                self.closes_for_scope(guid)?
            }
            (Kind::Page, "crash") => {
                if !params.is_object() {
                    return Err(ProjectionError::Invalid("crash event params"));
                }
                vec![Notice::PageCrashed(entry.id(guid))]
            }
            _ => Vec::new(),
        };
        self.publish(&notices)?;
        self.mark_closed(&notices);
        Ok(())
    }

    fn request_notice(
        &self,
        context_guid: &str,
        params: &Value,
    ) -> Result<Vec<Notice>, ProjectionError> {
        let source = object(params)?;
        let request_guid = channel_guid(source, "request")?;
        let request = self.entry_as(request_guid, "Request")?;
        self.require_context(request_guid, context_guid)?;
        let Some(page) = self.event_page(source, context_guid, request_guid)? else {
            return Ok(Vec::new());
        };
        let Kind::Request(snapshot) = &request.kind else {
            unreachable!("entry_as checked Request")
        };
        Ok(vec![Notice::Request {
            page,
            request: request.id(request_guid),
            snapshot: snapshot.clone(),
        }])
    }

    fn response_notice(
        &self,
        context_guid: &str,
        params: &Value,
    ) -> Result<Vec<Notice>, ProjectionError> {
        let source = object(params)?;
        let response_guid = channel_guid(source, "response")?;
        let response = self.entry_as(response_guid, "Response")?;
        let Kind::Response(snapshot) = &response.kind else {
            unreachable!("entry_as checked Response")
        };
        let request = self.entry_as(&snapshot.request.guid, "Request")?;
        if request.generation != snapshot.request.generation {
            return Err(ProjectionError::Invalid("recycled Response request"));
        }
        self.require_context(&snapshot.request.guid, context_guid)?;
        let Some(page) = self.event_page(source, context_guid, &snapshot.request.guid)? else {
            return Ok(Vec::new());
        };
        let Kind::Request(request_snapshot) = &request.kind else {
            unreachable!("entry_as checked Request")
        };
        Ok(vec![Notice::Response {
            page,
            response: response.id(response_guid),
            snapshot: snapshot.clone(),
            request_snapshot: request_snapshot.clone(),
        }])
    }

    fn download_notice(
        &self,
        page_guid: &str,
        params: &Value,
    ) -> Result<Vec<Notice>, ProjectionError> {
        let source = object(params)?;
        let artifact_guid = channel_guid(source, "artifact")?;
        let artifact = self.entry_as(artifact_guid, "Artifact")?;
        let page = self.entry_as(page_guid, "Page")?;
        if page.closed {
            return Err(ProjectionError::Invalid("download from closed Page"));
        }
        let context_guid = page
            .parent
            .as_deref()
            .ok_or(ProjectionError::Invalid("Page context"))?;
        if artifact.parent.as_deref() != Some(context_guid) {
            return Err(ProjectionError::Invalid("download Artifact context"));
        }
        Ok(vec![Notice::Download {
            page: page.id(page_guid),
            artifact: artifact.id(artifact_guid),
            url: field_string(source, "url", MAX_URL_BYTES)?,
            suggested_filename: field_string(source, "suggestedFilename", MAX_FILENAME_BYTES)?,
        }])
    }

    fn event_page(
        &self,
        source: &Map<String, Value>,
        context_guid: &str,
        request_guid: &str,
    ) -> Result<Option<ObjectId>, ProjectionError> {
        let Some(value) = source.get("page") else {
            return Ok(None);
        };
        let page_guid = channel_value(value)?;
        let page = self.entry_as(page_guid, "Page")?;
        if page.closed || page.parent.as_deref() != Some(context_guid) {
            return Err(ProjectionError::Invalid("network Page context"));
        }
        if let Some(owner_page) =
            self.ancestor_guid(request_guid, |kind| matches!(kind, Kind::Page))?
            && owner_page != page_guid
        {
            return Err(ProjectionError::Invalid("network Request Page"));
        }
        Ok(Some(page.id(page_guid)))
    }

    fn require_context(&self, guid: &str, expected: &str) -> Result<(), ProjectionError> {
        let actual = self.ancestor_guid(guid, |kind| matches!(kind, Kind::Context))?;
        if actual.as_deref() == Some(expected) {
            Ok(())
        } else {
            Err(ProjectionError::Invalid("network Context lineage"))
        }
    }

    fn entry_as(&self, guid: &str, type_name: &str) -> Result<&Entry, ProjectionError> {
        valid_guid(guid)?;
        let entry = self
            .objects
            .get(guid)
            .ok_or(ProjectionError::Invalid("unknown channel GUID"))?;
        let matches = match type_name {
            "Page" => matches!(entry.kind, Kind::Page),
            "Request" => matches!(entry.kind, Kind::Request(_)),
            "Response" => matches!(entry.kind, Kind::Response(_)),
            "Artifact" => matches!(entry.kind, Kind::Artifact),
            _ => false,
        };
        if matches {
            Ok(entry)
        } else {
            Err(ProjectionError::Invalid("channel type"))
        }
    }

    fn ancestor_guid(
        &self,
        guid: &str,
        predicate: impl Fn(&Kind) -> bool,
    ) -> Result<Option<String>, ProjectionError> {
        let mut current = Some(guid);
        for _ in 0..self.objects.len() {
            let Some(candidate) = current else {
                return Ok(None);
            };
            let entry = self
                .objects
                .get(candidate)
                .ok_or(ProjectionError::Invalid("missing ancestor"))?;
            if predicate(&entry.kind) {
                return Ok(Some(candidate.to_owned()));
            }
            current = entry.parent.as_deref();
        }
        Err(ProjectionError::Invalid("ancestor cycle"))
    }

    fn has_ancestor(&self, guid: &str, ancestor: &str) -> Result<bool, ProjectionError> {
        let mut current = Some(guid);
        for _ in 0..self.objects.len() {
            let Some(candidate) = current else {
                return Ok(false);
            };
            if candidate == ancestor {
                return Ok(true);
            }
            current = self
                .objects
                .get(candidate)
                .ok_or(ProjectionError::Invalid("missing ancestor"))?
                .parent
                .as_deref();
        }
        Err(ProjectionError::Invalid("ancestor cycle"))
    }

    fn closes_for_scope(&self, guid: &str) -> Result<Vec<Notice>, ProjectionError> {
        let mut pages = Vec::new();
        let mut contexts = Vec::new();
        let mut browsers = Vec::new();
        for (candidate, entry) in &self.objects {
            if entry.closed || !self.has_ancestor(candidate, guid)? {
                continue;
            }
            match entry.kind {
                Kind::Page => pages.push(entry.id(candidate)),
                Kind::Context => contexts.push(entry.id(candidate)),
                Kind::Browser => browsers.push(entry.id(candidate)),
                _ => {}
            }
        }
        Ok(ordered_closes(pages, contexts, browsers))
    }

    fn closes_for_guids(&self, guids: &BTreeSet<&str>) -> Vec<Notice> {
        let mut pages = Vec::new();
        let mut contexts = Vec::new();
        let mut browsers = Vec::new();
        for guid in guids {
            let entry = self.objects.get(*guid).expect("checked disposal GUID");
            if entry.closed {
                continue;
            }
            match entry.kind {
                Kind::Page => pages.push(entry.id(guid)),
                Kind::Context => contexts.push(entry.id(guid)),
                Kind::Browser => browsers.push(entry.id(guid)),
                _ => {}
            }
        }
        ordered_closes(pages, contexts, browsers)
    }

    fn closes_for_all(&self) -> Vec<Notice> {
        let mut pages = Vec::new();
        let mut contexts = Vec::new();
        let mut browsers = Vec::new();
        for (guid, entry) in &self.objects {
            if entry.closed {
                continue;
            }
            match entry.kind {
                Kind::Page => pages.push(entry.id(guid)),
                Kind::Context => contexts.push(entry.id(guid)),
                Kind::Browser => browsers.push(entry.id(guid)),
                _ => {}
            }
        }
        ordered_closes(pages, contexts, browsers)
    }

    fn mark_closed(&mut self, notices: &[Notice]) {
        for notice in notices {
            let id = match notice {
                Notice::PageClosed(id) | Notice::ContextClosed(id) | Notice::BrowserClosed(id) => {
                    id
                }
                _ => continue,
            };
            if let Some(entry) = self.objects.get_mut(&id.guid)
                && entry.generation == id.generation
            {
                entry.closed = true;
            }
        }
    }

    fn publish(&mut self, notices: &[Notice]) -> Result<(), ProjectionError> {
        let count = self
            .notices
            .len()
            .checked_add(notices.len())
            .filter(|value| *value <= MAX_QUEUED_NOTICES)
            .ok_or(ProjectionError::NoticeLimit)?;
        let mut total = self.notice_bytes;
        for notice in notices {
            total = total
                .checked_add(notice_bytes(notice))
                .filter(|value| *value <= MAX_QUEUED_NOTICE_BYTES)
                .ok_or(ProjectionError::NoticeLimit)?;
        }
        self.notices
            .try_reserve(count - self.notices.len())
            .map_err(|_| ProjectionError::NoticeLimit)?;
        for notice in notices {
            self.notices.push_back(QueuedNotice {
                notice: notice.clone(),
                bytes: notice_bytes(notice),
            });
        }
        self.notice_bytes = total;
        Ok(())
    }
}

fn ordered_closes(
    mut pages: Vec<ObjectId>,
    mut contexts: Vec<ObjectId>,
    mut browsers: Vec<ObjectId>,
) -> Vec<Notice> {
    pages.sort_by_key(|id| id.generation);
    contexts.sort_by_key(|id| id.generation);
    browsers.sort_by_key(|id| id.generation);
    pages
        .into_iter()
        .map(Notice::PageClosed)
        .chain(contexts.into_iter().map(Notice::ContextClosed))
        .chain(browsers.into_iter().map(Notice::BrowserClosed))
        .collect()
}

fn object(value: &Value) -> Result<&Map<String, Value>, ProjectionError> {
    value
        .as_object()
        .ok_or(ProjectionError::Invalid("event or initializer object"))
}

fn field_string(
    source: &Map<String, Value>,
    name: &'static str,
    maximum: usize,
) -> Result<String, ProjectionError> {
    let value = source
        .get(name)
        .and_then(Value::as_str)
        .ok_or(ProjectionError::Invalid(name))?;
    if value.len() > maximum {
        return Err(ProjectionError::MetadataLimit);
    }
    Ok(value.to_owned())
}

fn channel_guid<'a>(
    source: &'a Map<String, Value>,
    name: &'static str,
) -> Result<&'a str, ProjectionError> {
    let value = source.get(name).ok_or(ProjectionError::Invalid(name))?;
    channel_value(value)
}

fn channel_value(value: &Value) -> Result<&str, ProjectionError> {
    let guid = value
        .as_object()
        .and_then(|value| value.get("guid"))
        .and_then(Value::as_str)
        .ok_or(ProjectionError::Invalid("channel GUID"))?;
    valid_guid(guid)?;
    Ok(guid)
}

fn valid_guid(guid: &str) -> Result<(), ProjectionError> {
    if guid.is_empty() || guid.len() > MAX_GUID_BYTES || guid.chars().any(char::is_control) {
        Err(ProjectionError::Invalid("GUID syntax"))
    } else {
        Ok(())
    }
}

fn parse_headers(source: &Map<String, Value>) -> Result<Vec<Header>, ProjectionError> {
    let values = source
        .get("headers")
        .and_then(Value::as_array)
        .ok_or(ProjectionError::Invalid("headers"))?;
    if values.len() > MAX_HEADERS {
        return Err(ProjectionError::MetadataLimit);
    }
    let mut headers = Vec::with_capacity(values.len());
    let mut bytes = 0usize;
    for value in values {
        let value = object(value)?;
        let name = field_string(value, "name", MAX_HEADER_BYTES)?;
        let header_value = field_string(value, "value", MAX_HEADER_BYTES)?;
        bytes = bytes
            .checked_add(name.len() + header_value.len() + 32)
            .filter(|value| *value <= MAX_HEADER_BYTES)
            .ok_or(ProjectionError::MetadataLimit)?;
        headers.push(Header {
            name,
            value: header_value,
        });
    }
    Ok(headers)
}

fn parse_request(initializer: &Value) -> Result<RequestSnapshot, ProjectionError> {
    let source = object(initializer)?;
    Ok(RequestSnapshot {
        url: field_string(source, "url", MAX_URL_BYTES)?,
        method: field_string(source, "method", 64)?,
        resource_type: field_string(source, "resourceType", 64)?,
        headers: parse_headers(source)?,
        is_navigation_request: source
            .get("isNavigationRequest")
            .and_then(Value::as_bool)
            .ok_or(ProjectionError::Invalid("navigation request flag"))?,
    })
}

fn request_bytes(value: &RequestSnapshot) -> usize {
    value.url.len()
        + value.method.len()
        + value.resource_type.len()
        + value
            .headers
            .iter()
            .map(|header| header.name.len() + header.value.len() + 32)
            .sum::<usize>()
        + 128
}

fn response_bytes(value: &ResponseSnapshot) -> usize {
    value.url.len()
        + value.status_text.len()
        + value.request.guid.len()
        + value
            .headers
            .iter()
            .map(|header| header.name.len() + header.value.len() + 32)
            .sum::<usize>()
        + 128
}

fn notice_bytes(notice: &Notice) -> usize {
    NOTICE_OVERHEAD
        + match notice {
            Notice::Request {
                page,
                request,
                snapshot,
            } => page.guid.len() + request.guid.len() + request_bytes(snapshot),
            Notice::Response {
                page,
                response,
                snapshot,
                request_snapshot,
            } => {
                page.guid.len()
                    + response.guid.len()
                    + response_bytes(snapshot)
                    + request_bytes(request_snapshot)
            }
            Notice::Download {
                page,
                artifact,
                url,
                suggested_filename,
            } => page.guid.len() + artifact.guid.len() + url.len() + suggested_filename.len(),
            Notice::PageClosed(id)
            | Notice::PageCrashed(id)
            | Notice::ContextClosed(id)
            | Notice::BrowserClosed(id) => id.guid.len(),
            Notice::Disconnected => 0,
        }
}

#[cfg(test)]
mod tests;
