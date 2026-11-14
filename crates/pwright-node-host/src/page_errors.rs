//! Optional Page error publication over the existing ordered lifecycle fence.
//! This leaf observes native Runtime exceptions only; it installs no task,
//! transport, browser process, polling loop, or language-owned queue.

use std::collections::VecDeque;
use std::error::Error;
use std::fmt::Write;
use std::sync::Arc;

use pwright_backend_cdp_topology::CdpTopology;
use pwright_cdp::{CdpEvent, generated::page};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use pwright_plugin_chromium_frame_observer::{ChromiumFrameEventObserver, FrameObservationResult};
use serde_json::Value;

use crate::frame_metadata::{
    MAX_PAGE_ERROR_MESSAGE_BYTES, MAX_PAGE_ERROR_NAME_BYTES, MAX_PAGE_ERROR_STACK_BYTES,
};
use crate::frame_trees::FrameTrees;
#[cfg(feature = "worker-error-events")]
use crate::worker_events::Installation as WorkerInstallation;
use crate::{HostError, HostSession, PageError};

const MAX_PAGES: usize = 16;
pub(crate) const MAX_PAGE_ERRORS_PER_PAGE: usize = 200;
const MAX_PAGE_ERROR_HISTORY_ITEMS: u64 = (MAX_PAGES * MAX_PAGE_ERRORS_PER_PAGE) as u64;
const MAX_PAGE_ERROR_HISTORY_BYTES: u64 = 16 * 1024 * 1024;
const HISTORY_SCOPE: ResourceScope = ResourceScope::extension(175, 1);

struct StoredError {
    sequence: u64,
    error: Arc<PageError>,
    _lease: ResourceLease,
}

struct PageHistory {
    page: Option<ObjectId>,
    errors: VecDeque<StoredError>,
}

impl PageHistory {
    fn new() -> Result<Self, HostError> {
        let mut errors = VecDeque::new();
        errors
            .try_reserve_exact(MAX_PAGE_ERRORS_PER_PAGE)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(Self { page: None, errors })
    }

    fn clear(&mut self) {
        self.errors.clear();
        self.page = None;
    }
}

struct HistoryState {
    next_sequence: u64,
    pages: Vec<PageHistory>,
    closed: bool,
}

pub(crate) const fn requirements() -> ResourceRequirements {
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<HistoryState>()
        + MAX_PAGES
            * (std::mem::size_of::<PageHistory>()
                + MAX_PAGE_ERRORS_PER_PAGE * std::mem::size_of::<StoredError>()))
        as u64;
    ResourceRequirements::none()
        .with_class(
            ResourceClass::EventItem,
            MAX_PAGE_ERROR_HISTORY_ITEMS,
            MAX_PAGE_ERRORS_PER_PAGE as u64,
        )
        .with_class(
            ResourceClass::EventBytes,
            MAX_PAGE_ERROR_HISTORY_BYTES,
            MAX_PAGE_ERROR_HISTORY_BYTES,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes,
            storage_bytes,
        )
        .with_metadata(MAX_PAGES + 1, MAX_PAGE_ERROR_HISTORY_ITEMS as usize + 1)
}

pub(crate) struct Installation {
    resources: pwright_core::ResourceLedger,
    state: std::sync::Mutex<HistoryState>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let ledger = resources.claim(plan).map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                HISTORY_SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let mut pages = Vec::new();
        pages
            .try_reserve_exact(MAX_PAGES)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        for _ in 0..MAX_PAGES {
            pages.push(PageHistory::new()?);
        }
        Ok(Arc::new(Self {
            resources: ledger,
            state: std::sync::Mutex::new(HistoryState {
                next_sequence: 1,
                pages,
                closed: false,
            }),
            _storage: storage,
        }))
    }

    fn requests(storage_bytes: usize) -> Result<[ResourceRequest; 2], HostError> {
        let bytes = u64::try_from(storage_bytes)
            .map_err(|_| HostError::ResourceExhausted("Page error history size overflow".into()))?;
        if bytes > MAX_PAGE_ERROR_HISTORY_BYTES {
            return Err(HostError::ResourceExhausted(
                "Page error exceeds the history byte capacity".into(),
            ));
        }
        Ok([
            ResourceRequest::new(ResourceClass::EventItem, 1),
            ResourceRequest::new(ResourceClass::EventBytes, bytes),
        ])
    }

    fn oldest_page(state: &HistoryState) -> Option<usize> {
        state
            .pages
            .iter()
            .enumerate()
            .filter_map(|(index, page)| page.errors.front().map(|error| (index, error.sequence)))
            .min_by_key(|(_, sequence)| *sequence)
            .map(|(index, _)| index)
    }

    fn record(
        &self,
        page: ObjectId,
        storage_bytes: usize,
        error: Arc<PageError>,
    ) -> Result<(), HostError> {
        let requests = Self::requests(storage_bytes)?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        let page_index = state
            .pages
            .iter()
            .position(|history| history.page == Some(page))
            .or_else(|| {
                state
                    .pages
                    .iter()
                    .position(|history| history.page.is_none())
            })
            .ok_or_else(|| {
                HostError::ResourceExhausted(format!(
                    "Page error history exceeds {MAX_PAGES} live Pages"
                ))
            })?;
        let sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| HostError::ResourceExhausted("Page error sequence exhausted".into()))?;
        let sequence = std::mem::replace(&mut state.next_sequence, sequence);
        let scope = ResourceScope::Object(page);

        if state.pages[page_index].errors.len() == MAX_PAGE_ERRORS_PER_PAGE {
            let mut oldest = state.pages[page_index]
                .errors
                .pop_front()
                .expect("a full Page error history has an oldest item");
            if let Err(error) = oldest._lease.try_replace(scope, &requests) {
                state.pages[page_index].errors.push_front(oldest);
                state.next_sequence = sequence;
                return Err(HostError::ResourceExhausted(error.to_string()));
            }
            state.pages[page_index].errors.push_back(StoredError {
                sequence,
                error,
                _lease: oldest._lease,
            });
            return Ok(());
        }

        let lease = loop {
            match self.resources.reserve_many(scope, &requests) {
                Ok(lease) => break lease,
                Err(error) => {
                    let Some(oldest_page) = Self::oldest_page(&state) else {
                        state.next_sequence = sequence;
                        return Err(HostError::ResourceExhausted(error.to_string()));
                    };
                    state.pages[oldest_page].errors.pop_front();
                }
            }
        };
        state.pages[page_index].page = Some(page);
        state.pages[page_index].errors.push_back(StoredError {
            sequence,
            error,
            _lease: lease,
        });
        Ok(())
    }

    fn snapshot(&self, page: ObjectId) -> Result<Vec<Arc<PageError>>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(state
            .pages
            .iter()
            .find(|history| history.page == Some(page))
            .map(|history| {
                history
                    .errors
                    .iter()
                    .map(|stored| Arc::clone(&stored.error))
                    .collect()
            })
            .unwrap_or_default())
    }

    fn retain(&self, topology: &CdpTopology) -> Result<(), HostError> {
        let mut state = self.state.lock().map_err(HostError::backend)?;
        for history in &mut state.pages {
            if history.page.is_some_and(|page| {
                !topology
                    .page_session_routes()
                    .any(|route| route.object_id() == page)
            }) {
                history.clear();
            }
        }
        Ok(())
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        for history in &mut state.pages {
            history.clear();
        }
    }
}

pub(crate) struct Observer {
    trees: Arc<FrameTrees>,
    history: Arc<Installation>,
    #[cfg(feature = "worker-error-events")]
    workers: Arc<WorkerInstallation>,
    downstream: Arc<dyn ChromiumFrameEventObserver>,
}

impl Observer {
    pub(crate) fn new(
        trees: Arc<FrameTrees>,
        history: Arc<Installation>,
        #[cfg(feature = "worker-error-events")] workers: Arc<WorkerInstallation>,
        downstream: Arc<dyn ChromiumFrameEventObserver>,
    ) -> Self {
        Self {
            trees,
            history,
            #[cfg(feature = "worker-error-events")]
            workers,
            downstream,
        }
    }
}

fn page_for_exception(
    frame_page: Option<ObjectId>,
    target_page: Option<ObjectId>,
) -> Option<ObjectId> {
    frame_page.or(target_page)
}

fn invalid(field: &str) -> Box<dyn Error + Send + Sync> {
    Box::new(HostError::Backend(format!(
        "Runtime exception event is missing {field}"
    )))
}

fn push_bounded(output: &mut String, value: &str) -> Result<(), HostError> {
    let length = output
        .len()
        .checked_add(value.len())
        .ok_or_else(|| HostError::ResourceExhausted("Page error stack length overflow".into()))?;
    if length > MAX_PAGE_ERROR_STACK_BYTES {
        return Err(HostError::ResourceExhausted(
            "Page error stack exceeds byte limit".into(),
        ));
    }
    output.push_str(value);
    Ok(())
}

fn javascript_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_owned(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn remote_object_string(exception: &serde_json::Map<String, Value>) -> String {
    if let Some(value) = exception.get("value") {
        return javascript_string(value);
    }
    if let Some(value) = exception.get("unserializableValue").and_then(Value::as_str) {
        return value.to_owned();
    }
    if exception.get("type").and_then(Value::as_str) == Some("undefined") {
        return "undefined".to_owned();
    }
    exception
        .get("className")
        .and_then(Value::as_str)
        .or_else(|| exception.get("type").and_then(Value::as_str))
        .unwrap_or("")
        .to_owned()
}

fn exception_message(details: &serde_json::Map<String, Value>) -> Result<String, HostError> {
    if let Some(exception) = details.get("exception").and_then(Value::as_object) {
        let message = exception
            .get("description")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| remote_object_string(exception));
        if message.len() > MAX_PAGE_ERROR_STACK_BYTES {
            return Err(HostError::ResourceExhausted(
                "Page error stack exceeds byte limit".into(),
            ));
        }
        return Ok(message);
    }

    let text = details
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| HostError::Backend("Runtime exception is missing text".into()))?;
    let mut message = String::new();
    push_bounded(&mut message, text)?;
    if let Some(frames) = details
        .get("stackTrace")
        .and_then(|value| value.get("callFrames"))
        .and_then(Value::as_array)
    {
        for frame in frames {
            let frame = frame.as_object().ok_or_else(|| {
                HostError::Backend("Runtime exception call frame is invalid".into())
            })?;
            let function = frame
                .get("functionName")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .unwrap_or("<anonymous>");
            let url = frame.get("url").and_then(Value::as_str).unwrap_or("");
            let line = frame
                .get("lineNumber")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    HostError::Backend("Runtime exception frame is missing line".into())
                })?;
            let column = frame
                .get("columnNumber")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    HostError::Backend("Runtime exception frame is missing column".into())
                })?;
            let mut location = String::new();
            write!(&mut location, "\n    at {function} ({url}:{line}:{column})")
                .map_err(HostError::backend)?;
            push_bounded(&mut message, &location)?;
        }
    }
    Ok(message)
}

fn name_override(details: &serde_json::Map<String, Value>) -> Option<&str> {
    details
        .get("exception")
        .and_then(Value::as_object)
        .and_then(|exception| exception.get("preview"))
        .and_then(Value::as_object)
        .and_then(|preview| preview.get("properties"))
        .and_then(Value::as_array)
        .and_then(|properties| {
            properties.iter().find_map(|property| {
                let property = property.as_object()?;
                (property.get("name").and_then(Value::as_str) == Some("name")).then(|| {
                    property
                        .get("value")
                        .and_then(Value::as_str)
                        .unwrap_or("Error")
                })
            })
        })
}

struct PageErrorPlan {
    name: String,
    message: String,
    stack: String,
    storage_bytes: usize,
}

impl PageErrorPlan {
    fn new(details: &serde_json::Map<String, Value>) -> Result<Self, HostError> {
        let message_with_stack = exception_message(details)?;
        let lines = message_with_stack.split('\n').collect::<Vec<_>>();
        let first_stack = lines.iter().position(|line| line.starts_with("    at"));
        let (message_with_name, stack) = match first_stack {
            Some(index) => (lines[..index].join("\n"), message_with_stack),
            None => (message_with_stack, String::new()),
        };
        let (default_name, message) = match message_with_name.find(':') {
            Some(index) => {
                let start = usize::min(index + 2, message_with_name.len());
                (&message_with_name[..index], &message_with_name[start..])
            }
            None => ("", message_with_name.as_str()),
        };
        let name = name_override(details).unwrap_or(default_name).to_owned();
        let message = message.to_owned();
        if name.len() > MAX_PAGE_ERROR_NAME_BYTES {
            return Err(HostError::ResourceExhausted(
                "Page error name exceeds byte limit".into(),
            ));
        }
        if message.len() > MAX_PAGE_ERROR_MESSAGE_BYTES {
            return Err(HostError::ResourceExhausted(
                "Page error message exceeds byte limit".into(),
            ));
        }
        let storage_bytes = PageError::storage_bytes(name.len(), message.len(), stack.len())?;
        Ok(Self {
            name,
            message,
            stack,
            storage_bytes,
        })
    }

    const fn storage_bytes(&self) -> usize {
        self.storage_bytes
    }

    fn build(self) -> Result<PageError, HostError> {
        PageError::new(self.name, self.message, self.stack)
    }
}

impl ChromiumFrameEventObserver for Observer {
    fn observes_page(&self, page: ObjectId) -> bool {
        self.downstream.observes_page(page)
    }

    fn prepare_page(&self, topology: &CdpTopology, page: ObjectId) -> FrameObservationResult {
        self.downstream.prepare_page(topology, page)
    }

    fn discard_prepared_page(
        &self,
        topology: &CdpTopology,
        page: ObjectId,
    ) -> FrameObservationResult {
        self.downstream.discard_prepared_page(topology, page)
    }

    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult {
        self.downstream.initialize_frame(topology, session, frame)
    }

    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult {
        self.downstream.reconcile(topology)?;
        self.history
            .retain(topology)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult {
        self.downstream.event(topology, event)?;
        if event.method != "Runtime.exceptionThrown" {
            return Ok(());
        }
        let session_id = event
            .session_id
            .as_deref()
            .ok_or_else(|| invalid("sessionId"))?;
        let details = event
            .params
            .get("exceptionDetails")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("exceptionDetails"))?;
        let execution_context_id = details.get("executionContextId").and_then(Value::as_i64);
        let frame_page = execution_context_id.and_then(|execution_context_id| {
            topology
                .frame_execution_routes()
                .find(|route| {
                    route.session_id() == session_id
                        && route.execution_context_id() == execution_context_id
                })
                .and_then(|route| topology.page_object_id_for_frame(route.object_id()))
        });
        let target_page = topology
            .target_session_route(session_id)
            .and_then(|target| topology.page_object_id(target.page_target_id()));
        let page = page_for_exception(frame_page, target_page);
        #[cfg(feature = "worker-error-events")]
        let page = match page {
            Some(page) => Some(page),
            None => self
                .workers
                .owner(session_id)
                .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?
                .map(|owner| owner.page()),
        };
        let Some(page) = page else {
            return Ok(());
        };
        let plan = PageErrorPlan::new(details)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        let storage_bytes = plan.storage_bytes();
        let error = Arc::new(
            plan.build()
                .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?,
        );
        self.trees
            .publish_page_error(page, storage_bytes, || Ok(Arc::clone(&error)))
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)?;
        self.history
            .record(page, storage_bytes, error)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    fn seal(&self) {
        self.history.close();
        self.downstream.seal();
    }
}

impl HostSession {
    pub fn page_errors(&self, page_id: &str) -> Result<Vec<Arc<PageError>>, HostError> {
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        self.require_visible_page_id(page)?;
        let history = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .as_ref()
            .ok_or(HostError::TargetClosed)?
            .page_errors
            .clone();
        self.frame_trees.get(page)?;
        history.snapshot(page)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_core::{ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits};
    use serde_json::json;

    use super::{
        Installation, MAX_PAGE_ERRORS_PER_PAGE, PageErrorPlan, page_for_exception, requirements,
    };
    use crate::PageError;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    fn error(message: String) -> (Arc<PageError>, usize) {
        let storage = PageError::storage_bytes("Error".len(), message.len(), 0).unwrap();
        (
            Arc::new(PageError::new("Error".into(), message, String::new()).unwrap()),
            storage,
        )
    }

    #[test]
    fn history_retains_the_last_two_hundred_errors_without_over_admission() {
        // Arrange
        let resources = resources();
        let history = Installation::new(&resources).unwrap();
        let page = ObjectId::new(7);

        // Act
        for index in 0..=MAX_PAGE_ERRORS_PER_PAGE {
            let (error, storage) = error(format!("error{index}"));
            history.record(page, storage, error).unwrap();
        }
        let errors = history.snapshot(page).unwrap();
        let metrics = resources.metrics().unwrap();

        // Assert
        assert_eq!(errors.len(), MAX_PAGE_ERRORS_PER_PAGE);
        assert_eq!(errors.first().unwrap().message(), "error1");
        assert_eq!(errors.last().unwrap().message(), "error200");
        assert_eq!(
            metrics.class(ResourceClass::EventItem).current,
            MAX_PAGE_ERRORS_PER_PAGE as u64
        );
        assert_eq!(metrics.rejected_reservations, 0);
    }

    #[test]
    fn close_releases_history_and_rejects_future_reads() {
        // Arrange
        let resources = resources();
        let history = Installation::new(&resources).unwrap();
        let page = ObjectId::new(7);
        let (error, storage) = error("broken".into());
        history.record(page, storage, error).unwrap();

        // Act
        history.close();
        let result = history.snapshot(page);
        let metrics = resources.metrics().unwrap();

        // Assert
        assert!(matches!(result, Err(crate::HostError::TargetClosed)));
        assert_eq!(metrics.class(ResourceClass::EventItem).current, 0);
        assert_eq!(metrics.class(ResourceClass::EventBytes).current, 0);
    }

    #[test]
    fn error_description_preserves_stack_and_custom_name() {
        // Arrange
        let details = json!({
            "exception": {
                "description": "Error: Fancy error!\n    at c (myscript.js:14:11)",
                "preview": {"properties": [{"name": "name", "value": "my-name"}]}
            }
        });

        // Act
        let error = PageErrorPlan::new(details.as_object().unwrap())
            .unwrap()
            .build()
            .unwrap();

        // Assert
        assert_eq!(error.name(), "my-name");
        assert_eq!(error.message(), "Fancy error!");
        assert_eq!(
            error.stack(),
            "Error: Fancy error!\n    at c (myscript.js:14:11)"
        );
    }

    #[test]
    fn primitive_thrown_values_match_javascript_string_conversion() {
        // Arrange
        let details = [
            json!({"exception": {"type": "object", "value": null}}),
            json!({"exception": {"type": "undefined"}}),
            json!({"exception": {"type": "number", "value": 0}}),
            json!({"exception": {"type": "string", "value": ""}}),
        ];

        // Act
        let messages = details.map(|details| {
            PageErrorPlan::new(details.as_object().unwrap())
                .unwrap()
                .build()
                .unwrap()
                .message()
                .to_owned()
        });

        // Assert
        assert_eq!(messages, ["null", "undefined", "0", ""]);
    }

    #[test]
    fn object_description_is_the_public_message() {
        // Arrange
        let details = json!({"exception": {"type": "object", "description": "Object"}});

        // Act
        let error = PageErrorPlan::new(details.as_object().unwrap())
            .unwrap()
            .build()
            .unwrap();

        // Assert
        assert_eq!(error.name(), "");
        assert_eq!(error.message(), "Object");
        assert_eq!(error.stack(), "");
    }

    #[test]
    fn details_without_remote_object_build_a_bounded_stack() {
        // Arrange
        let details = json!({
            "text": "Uncaught",
            "stackTrace": {"callFrames": [{
                "functionName": "boom",
                "url": "app.js",
                "lineNumber": 3,
                "columnNumber": 7
            }]}
        });

        // Act
        let error = PageErrorPlan::new(details.as_object().unwrap())
            .unwrap()
            .build()
            .unwrap();

        // Assert
        assert_eq!(error.message(), "Uncaught");
        assert_eq!(error.stack(), "Uncaught\n    at boom (app.js:3:7)");
    }

    #[test]
    fn frame_page_precedes_target_page_for_exception_ownership() {
        // Arrange
        let frame_page = ObjectId::new(1);
        let target_page = ObjectId::new(2);

        // Act
        let selections = [
            page_for_exception(Some(frame_page), Some(target_page)),
            page_for_exception(None, Some(target_page)),
            page_for_exception(None, None),
        ];

        // Assert
        assert_eq!(selections, [Some(frame_page), Some(target_page), None]);
    }
}
